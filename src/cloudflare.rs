//! Minimal Cloudflare API client for the Worker's own account operations.
//!
//! Used by [`crate::email::onboarding`] to enable Email Routing on a tenant's
//! zone and point its catch-all at this Worker.
//!
//! Credentials: the token is not an environment variable -- it lives in the
//! account Secrets Store and is bound to this Worker by name (`CF_API_TOKEN`;
//! see `tf/cloudflare/sites/secrets.tf`). Reading it is asynchronous, so
//! [`api_token`] resolves it once per call. Onboarding is a rare, operator-
//! adjacent path, so there is no caching here.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use worker::*;

#[wasm_bindgen]
extern "C" {
    /// A Workers Secrets Store binding. `get()` resolves to the secret value.
    type SecretsStoreSecret;

    #[wasm_bindgen(method, catch)]
    fn get(this: &SecretsStoreSecret) -> std::result::Result<js_sys::Promise, JsValue>;
}

/// Resolve the Cloudflare API token from the `CF_API_TOKEN` binding.
pub async fn api_token(env: &Env) -> Result<String> {
    let env_js: JsValue = env.clone().into();
    let binding = js_sys::Reflect::get(&env_js, &"CF_API_TOKEN".into())
        .map_err(|_| Error::from("reflecting CF_API_TOKEN"))?;
    if binding.is_undefined() || binding.is_null() {
        return Err(Error::from(
            "CF_API_TOKEN binding is not configured on this Worker",
        ));
    }
    let secret: SecretsStoreSecret = binding.unchecked_into();
    let promise = secret
        .get()
        .map_err(|e| Error::from(format!("secrets store get: {e:?}")))?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| Error::from(format!("secrets store read failed: {e:?}")))?;
    value
        .as_string()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::from("CF_API_TOKEN resolved to an empty value"))
}

/// The account id the Worker's API calls are scoped to, from the
/// (non-secret) `CLOUDFLARE_ACCOUNT_ID` var.
///
/// Unused until the Email Sending domain onboarding lands (account-scoped);
/// retained here because it is the natural companion to [`api_token`].
#[allow(dead_code)]
pub fn account_id(env: &Env) -> Result<String> {
    env.var("CLOUDFLARE_ACCOUNT_ID")
        .map(|v| v.to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::from("CLOUDFLARE_ACCOUNT_ID is not set"))
}

/// Call the Cloudflare API v4 and return the `result` field on success.
///
/// `path` is everything after `/client/v4`, e.g. `/zones?name=acme.com`.
/// A non-2xx status, or a body with `success: false`, becomes an `Err`
/// carrying the API's own `errors` array.
pub async fn request(
    env: &Env,
    method: Method,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<serde_json::Value> {
    let token = api_token(env).await?;
    let url = format!("https://api.cloudflare.com/client/v4{path}");

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {token}"))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(method).with_headers(headers);
    if let Some(value) = body {
        let text = serde_json::to_string(&value)
            .map_err(|e| Error::from(format!("serializing request body: {e}")))?;
        init.with_body(Some(JsValue::from_str(&text)));
    }

    let req = Request::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(req).send().await?;
    let status = resp.status_code();
    let text = resp.text().await?;

    let json: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let success = json
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !(200..300).contains(&status) || !success {
        let detail = json
            .get("errors")
            .map(|e| e.to_string())
            .unwrap_or_else(|| text.clone());
        return Err(Error::from(format!("Cloudflare API {status}: {detail}")));
    }

    Ok(json
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}
