//! Facebook data deletion callback handler

use worker::*;

use crate::helpers::generate_id;
use crate::storage::*;

/// Handle POST /data-deletion
pub async fn handle_data_deletion(mut req: Request, env: Env, method: Method) -> Result<Response> {
    if method != Method::Post {
        return Response::error("Method not allowed", 405);
    }

    let app_secret = env
        .secret("META_APP_SECRET")
        .map(|s| s.to_string())
        .unwrap_or_default();

    let form = req.form_data().await?;
    let signed_request = match form.get("signed_request") {
        Some(FormEntry::Field(sr)) => sr,
        _ => return Response::error("Missing signed_request", 400),
    };

    let parts: Vec<&str> = signed_request.split('.').collect();
    if parts.len() != 2 {
        return Response::error("Invalid signed_request format", 400);
    }

    // Verify signature (constant-time)
    if !app_secret.is_empty() {
        use subtle::ConstantTimeEq;
        let computed = crate::crypto::hmac_sha256_hex(app_secret.as_bytes(), parts[1].as_bytes())?;
        let sig = base64url_to_hex(parts[0])?;
        let valid: bool = computed.as_bytes().ct_eq(sig.as_bytes()).into();
        if !valid {
            return Response::error("Invalid signature", 403);
        }
    }

    // Decode payload
    let payload = parts[1].replace('-', "+").replace('_', "/");
    let padded = match payload.len() % 4 {
        2 => format!("{}==", payload),
        3 => format!("{}=", payload),
        _ => payload,
    };

    let decoded = base64_decode(&padded)?;
    let json: serde_json::Value = serde_json::from_slice(&decoded)
        .map_err(|e| Error::from(format!("Invalid payload JSON: {}", e)))?;

    let fb_user_id = json.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    if fb_user_id.is_empty() {
        return Response::error("Missing user_id", 400);
    }

    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    // Find and delete tenant by facebook_id
    if let Some(tenant) = get_tenant_by_facebook_id(&kv, fb_user_id).await? {
        delete_tenant_data(&kv, &db, &tenant.id).await?;
    }

    let confirmation_code = generate_id();
    let base_url = req
        .url()
        .map(|u| format!("{}://{}", u.scheme(), u.host_str().unwrap_or("localhost")))
        .unwrap_or_default();

    let response = serde_json::json!({
        "url": format!("{}/admin/settings", base_url),
        "confirmation_code": confirmation_code
    });

    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    Ok(Response::ok(response.to_string())?.with_headers(headers))
}

fn base64url_to_hex(input: &str) -> Result<String> {
    let b64 = input.replace('-', "+").replace('_', "/");
    let padded = match b64.len() % 4 {
        2 => format!("{}==", b64),
        3 => format!("{}=", b64),
        _ => b64,
    };
    let bytes = base64_decode(&padded)?;
    Ok(bytes.iter().map(|b| format!("{:02x}", b)).collect())
}

fn base64_decode(input: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|e| Error::from(format!("Invalid base64: {e}")))
}
