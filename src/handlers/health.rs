//! Health / connection-status checks.
//!
//! Every system the worker depends on — a third-party integration, a
//! Cloudflare binding, or a secret group — implements [`Component`]. The
//! login page gates feature buttons on [`Component::ready`], the
//! management health panel renders [`Component::shallow_check`] rows,
//! and the docs URL declared by each component flows through to the
//! row's "Docs ↗" pill on the same panel.
//!
//! Two depths:
//! - **shallow**: presence-only — each component is asked whether all
//!   of its [`Requirement`]s are satisfied (secrets / vars / bindings).
//!   Cheap, sync, runs on every hit.
//! - **deep**: additionally invokes the optional async probe on
//!   components that have one (D1 query, KV get, Discord API ping).
//!   Cached in KV for 60s so /manage doesn't hammer providers.

use serde::{Deserialize, Serialize};
use worker::*;

const DEEP_CACHE_KEY: &str = "health:deep:cache:v2";
const DEEP_CACHE_TTL_SECS: u64 = 60;

/// Concatenate the public docs base URL with a relative page path at
/// compile time, so every component's `docs_url()` reads as one literal
/// and we don't re-stringify per request.
macro_rules! concat_doc {
    ($page:literal) => {
        concat!("https://ananthb.github.io/concierge/", $page)
    };
}

// ============================================================================
// Types
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Warn,
    Error,
}

/// A single configuration item a [`Component`] needs to function. Each
/// variant carries the env-side name of the item; `satisfied(env)` is
/// the uniform "is it set" probe.
#[derive(Clone, Copy, Debug)]
pub enum Requirement {
    Secret(&'static str),
    Var(&'static str),
    Binding(&'static str),
}

impl Requirement {
    pub fn name(&self) -> &'static str {
        match self {
            Requirement::Secret(n) | Requirement::Var(n) | Requirement::Binding(n) => n,
        }
    }
    pub fn satisfied(&self, env: &Env) -> bool {
        match self {
            Requirement::Secret(n) => secret_set(env, n),
            Requirement::Var(n) => var_set(env, n),
            Requirement::Binding(n) => binding_present(env, n),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Check {
    pub name: String,
    pub status: Status,
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HealthReport {
    pub overall: Status,
    pub generated_at: String,
    pub deep: bool,
    pub checks: Vec<Check>,
}

/// Public health endpoint shape. Returns ONLY the rollup: no per-check
/// detail, no secret names. Detailed status lives on /manage (Cloudflare
/// Access protected).
#[derive(Serialize)]
struct PublicHealth {
    overall: Status,
    generated_at: String,
}

// ============================================================================
// Component trait
// ============================================================================

/// A named system the worker depends on. The trait carries enough
/// metadata for both the management panel (which renders every
/// component's status as a table row) and per-feature gates on
/// user-facing pages (which call `ready()` to decide whether to render
/// the button at all).
///
/// Components that need an async probe beyond presence — D1's SELECT 1,
/// KV's __healthcheck get, Discord's /users/@me ping — implement that
/// as an inherent `probe(env).await` method. [`run_checks`] dispatches
/// to the probe when one exists; everything else falls back to
/// `shallow_check`.
pub trait Component {
    fn name(&self) -> &'static str;
    fn requirements(&self) -> &'static [Requirement];
    fn docs_url(&self) -> Option<&'static str> {
        None
    }

    /// Cheap sync "is every requirement satisfied" — the canonical
    /// answer for feature-gate code paths.
    fn ready(&self, env: &Env) -> bool {
        self.requirements().iter().all(|r| r.satisfied(env))
    }

    /// Presence-only [`Check`]: surveys every requirement and reports
    /// the missing ones (if any). Carries the docs URL so renderers can
    /// link operators straight to the relevant page.
    fn shallow_check(&self, env: &Env) -> Check {
        let missing: Vec<&'static str> = self
            .requirements()
            .iter()
            .filter(|r| !r.satisfied(env))
            .map(|r| r.name())
            .collect();
        let docs_url = self.docs_url().map(String::from);
        if missing.is_empty() {
            let n = self.requirements().len();
            let detail = if n == 1 {
                "configured".to_string()
            } else {
                format!("{n} requirements set")
            };
            Check {
                name: self.name().into(),
                status: Status::Ok,
                detail,
                docs_url,
            }
        } else {
            Check {
                name: self.name().into(),
                status: Status::Error,
                detail: format!("missing: {}", missing.join(", ")),
                docs_url,
            }
        }
    }
}

// ============================================================================
// Concrete components
// ============================================================================

// ---- User-facing OAuth / messaging flows -----------------------------------

pub struct FacebookLogin;
impl Component for FacebookLogin {
    fn name(&self) -> &'static str {
        "Facebook login"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[
            Requirement::Secret("META_APP_ID"),
            Requirement::Secret("META_APP_SECRET"),
        ]
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some(concat_doc!("facebook-app-setup.html"))
    }
}

pub struct WhatsAppSignup;
impl Component for WhatsAppSignup {
    fn name(&self) -> &'static str {
        "WhatsApp signup"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[
            Requirement::Secret("META_APP_ID"),
            Requirement::Secret("META_APP_SECRET"),
            Requirement::Secret("WHATSAPP_ACCESS_TOKEN"),
            Requirement::Var("WHATSAPP_WABA_ID"),
            Requirement::Var("WHATSAPP_SIGNUP_CONFIG_ID"),
        ]
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some(concat_doc!("whatsapp.html"))
    }
}

pub struct InstagramMessaging;
impl Component for InstagramMessaging {
    fn name(&self) -> &'static str {
        "Instagram messaging"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[
            Requirement::Secret("META_APP_ID"),
            Requirement::Secret("META_APP_SECRET"),
            Requirement::Secret("INSTAGRAM_VERIFY_TOKEN"),
        ]
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some(concat_doc!("instagram.html"))
    }
}

pub struct Discord;
impl Component for Discord {
    fn name(&self) -> &'static str {
        "Discord"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[
            Requirement::Secret("DISCORD_APPLICATION_ID"),
            Requirement::Secret("DISCORD_PUBLIC_KEY"),
            Requirement::Secret("DISCORD_BOT_TOKEN"),
        ]
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some(concat_doc!("discord.html"))
    }
}

pub struct Razorpay;
impl Component for Razorpay {
    fn name(&self) -> &'static str {
        "Razorpay"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[
            Requirement::Secret("RAZORPAY_KEY_ID"),
            Requirement::Secret("RAZORPAY_KEY_SECRET"),
            Requirement::Secret("RAZORPAY_WEBHOOK_SECRET"),
        ]
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some(concat_doc!("billing.html"))
    }
}

pub struct GoogleOAuth;
impl Component for GoogleOAuth {
    fn name(&self) -> &'static str {
        "Google OAuth"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[
            Requirement::Secret("GOOGLE_OAUTH_CLIENT_ID"),
            Requirement::Secret("GOOGLE_OAUTH_CLIENT_SECRET"),
        ]
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some(concat_doc!("configuration.html"))
    }
}

pub struct EncryptionKey;
impl Component for EncryptionKey {
    fn name(&self) -> &'static str {
        "Encryption key"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[Requirement::Secret("ENCRYPTION_KEY")]
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some(concat_doc!("configuration.html"))
    }
}

// ---- Cloudflare bindings ---------------------------------------------------

pub struct AiBinding;
impl Component for AiBinding {
    fn name(&self) -> &'static str {
        "AI binding"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[Requirement::Binding("AI")]
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some(concat_doc!("deployment.html"))
    }
}

pub struct ReplyBufferBinding;
impl Component for ReplyBufferBinding {
    fn name(&self) -> &'static str {
        "REPLY_BUFFER binding"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[Requirement::Binding("REPLY_BUFFER")]
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some(concat_doc!("deployment.html"))
    }
}

pub struct EmailBinding;
impl Component for EmailBinding {
    fn name(&self) -> &'static str {
        "EMAIL binding"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[Requirement::Binding("EMAIL")]
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some(concat_doc!("email-routing.html"))
    }
}

pub struct D1Binding;
impl Component for D1Binding {
    fn name(&self) -> &'static str {
        "D1 (DB binding)"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[Requirement::Binding("DB")]
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some(concat_doc!("deployment.html"))
    }
}
impl D1Binding {
    /// Reachability probe: if the binding is present, runs `SELECT 1`.
    /// The shallow check covers the "missing binding" branch.
    pub async fn probe(env: &Env) -> Check {
        let mut check = D1Binding.shallow_check(env);
        if !matches!(check.status, Status::Ok) {
            return check;
        }
        let db = match env.d1("DB") {
            Ok(d) => d,
            Err(_) => {
                check.status = Status::Error;
                check.detail = "binding missing".into();
                return check;
            }
        };
        match db
            .prepare("SELECT 1 as ok")
            .first::<serde_json::Value>(None)
            .await
        {
            Ok(_) => check.detail = "reachable".into(),
            Err(e) => {
                check.status = Status::Error;
                check.detail = format!("query failed: {e}");
            }
        }
        check
    }
}

pub struct KvBinding;
impl Component for KvBinding {
    fn name(&self) -> &'static str {
        "KV (KV binding)"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[Requirement::Binding("KV")]
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some(concat_doc!("deployment.html"))
    }
}
impl KvBinding {
    /// Reachability probe: if the binding is present, GETs a
    /// likely-missing key (a `None` result is still "reachable").
    pub async fn probe(env: &Env) -> Check {
        let mut check = KvBinding.shallow_check(env);
        if !matches!(check.status, Status::Ok) {
            return check;
        }
        let kv = match env.kv("KV") {
            Ok(k) => k,
            Err(_) => {
                check.status = Status::Error;
                check.detail = "binding missing".into();
                return check;
            }
        };
        match kv.get("__healthcheck").text().await {
            Ok(_) => check.detail = "reachable".into(),
            Err(e) => {
                check.status = Status::Error;
                check.detail = format!("get failed: {e}");
            }
        }
        check
    }
}

/// Standalone deep probe row — pings `/users/@me` on Discord with the
/// bot token. Separate from the [`Discord`] component (which covers
/// "are the secrets set") because it tests a different fact: whether
/// Discord accepts our token.
pub struct DiscordReachable;
impl DiscordReachable {
    pub async fn probe(env: &Env) -> Check {
        let docs_url = Some(concat_doc!("discord.html").to_string());
        let name = "Discord bot reachable".to_string();
        let token = env
            .secret("DISCORD_BOT_TOKEN")
            .map(|s| s.to_string())
            .unwrap_or_default();
        if token.is_empty() {
            return Check {
                name,
                status: Status::Warn,
                detail: "DISCORD_BOT_TOKEN not set, skipped".into(),
                docs_url,
            };
        }
        let url = "https://discord.com/api/v10/users/@me";
        let headers = match Headers::new().with_set("Authorization", &format!("Bot {token}")) {
            Ok(h) => h,
            Err(_) => {
                return Check {
                    name,
                    status: Status::Error,
                    detail: "couldn't build auth header".into(),
                    docs_url,
                }
            }
        };
        let mut init = RequestInit::new();
        init.with_method(Method::Get).with_headers(headers);
        let req = match Request::new_with_init(url, &init) {
            Ok(r) => r,
            Err(e) => {
                return Check {
                    name,
                    status: Status::Error,
                    detail: format!("request build: {e}"),
                    docs_url,
                }
            }
        };
        match Fetch::Request(req).send().await {
            Ok(mut r) if r.status_code() == 200 => Check {
                name,
                status: Status::Ok,
                detail: r
                    .json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|v| v.get("username").and_then(|s| s.as_str()).map(String::from))
                    .map(|u| format!("authenticated as @{u}"))
                    .unwrap_or_else(|| "200 OK".into()),
                docs_url,
            },
            Ok(r) => Check {
                name,
                status: Status::Error,
                detail: format!("Discord returned HTTP {}", r.status_code()),
                docs_url,
            },
            Err(e) => Check {
                name,
                status: Status::Error,
                detail: format!("fetch failed: {e}"),
                docs_url,
            },
        }
    }
}

// ============================================================================
// Registry + public API
// ============================================================================

/// Components covered by the always-on shallow sweep, in render order.
/// D1 and KV aren't here — they have async probes and are pushed
/// separately by [`run_checks`].
const SHALLOW_COMPONENTS: &[&dyn Component] = &[
    &AiBinding,
    &ReplyBufferBinding,
    &EmailBinding,
    &GoogleOAuth,
    &EncryptionKey,
    &FacebookLogin,
    &WhatsAppSignup,
    &InstagramMessaging,
    &Discord,
    &Razorpay,
];

/// Components whose missing requirements take the worker offline. Used
/// at request entry to serve a maintenance page rather than letting the
/// user reach a broken OAuth redirect or a session that can't be
/// encrypted. Returns the env-side names of every missing requirement
/// across all essentials.
pub fn essentials_missing(env: &Env) -> Vec<&'static str> {
    let essentials: &[&dyn Component] = &[&EncryptionKey, &GoogleOAuth];
    essentials
        .iter()
        .flat_map(|c| {
            c.requirements()
                .iter()
                .filter(|r| !r.satisfied(env))
                .map(|r| r.name())
        })
        .collect()
}

pub async fn handle_health(_req: Request, env: Env) -> Result<Response> {
    let report = run_checks(&env, false).await;
    let public = PublicHealth {
        overall: report.overall,
        generated_at: report.generated_at,
    };
    let body = serde_json::to_string(&public)?;
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Cache-Control", "no-store")?;
    let status = match public.overall {
        Status::Ok => 200,
        Status::Warn => 200,
        Status::Error => 503,
    };
    Ok(Response::ok(body)?
        .with_status(status)
        .with_headers(headers))
}

/// Build a HealthReport. `deep=true` runs the optional async probes
/// (e.g. Discord ping) on top of the always-on bindings + shallow
/// sweep, and caches the combined report in KV for 60s.
pub async fn run_checks(env: &Env, deep: bool) -> HealthReport {
    let mut checks = Vec::new();

    // Bindings with probes — async even at shallow depth because the
    // probe IS the binding's real readiness signal.
    checks.push(D1Binding::probe(env).await);
    checks.push(KvBinding::probe(env).await);

    // Pure-presence components.
    for c in SHALLOW_COMPONENTS {
        checks.push(c.shallow_check(env));
    }

    if deep {
        let kv_ok = env.kv("KV").ok();
        if let Some(kv) = kv_ok.as_ref() {
            if let Ok(Some(cached)) = kv.get(DEEP_CACHE_KEY).text().await {
                if let Ok(report) = serde_json::from_str::<HealthReport>(&cached) {
                    return report;
                }
            }
        }
        checks.push(DiscordReachable::probe(env).await);

        let report = finalize(checks, deep);
        if let Some(kv) = kv_ok {
            if let Ok(s) = serde_json::to_string(&report) {
                let _ = kv
                    .put(DEEP_CACHE_KEY, s)
                    .and_then(|p| Ok(p.expiration_ttl(DEEP_CACHE_TTL_SECS)))
                    .and_then(|p| Ok(p.execute()))
                    .map(|f| async move { f.await });
            }
        }
        return report;
    }

    finalize(checks, deep)
}

fn finalize(checks: Vec<Check>, deep: bool) -> HealthReport {
    let overall = checks
        .iter()
        .map(|c| c.status)
        .fold(Status::Ok, |acc, s| match (acc, s) {
            (Status::Error, _) | (_, Status::Error) => Status::Error,
            (Status::Warn, _) | (_, Status::Warn) => Status::Warn,
            _ => Status::Ok,
        });
    HealthReport {
        overall,
        generated_at: crate::helpers::now_iso(),
        deep,
        checks,
    }
}

// ============================================================================
// Env-side presence helpers
// ============================================================================

fn secret_set(env: &Env, name: &str) -> bool {
    env.secret(name)
        .map(|s| !s.to_string().is_empty())
        .unwrap_or(false)
}

fn var_set(env: &Env, name: &str) -> bool {
    env.var(name)
        .map(|v| !v.to_string().is_empty())
        .unwrap_or(false)
}

/// Generic binding-presence check via JsValue reflection. Works for
/// every wrangler.toml binding type (D1, KV, AI, DO, send_email,
/// queue producers, etc.) without per-type accessor calls.
fn binding_present(env: &Env, name: &str) -> bool {
    use wasm_bindgen::JsValue;
    let env_js: JsValue = env.clone().into();
    js_sys::Reflect::get(&env_js, &JsValue::from_str(name))
        .map(|v| !v.is_undefined())
        .unwrap_or(false)
}

// ============================================================================
// Headers helper
// ============================================================================

trait HeadersWithSet {
    fn with_set(self, name: &str, value: &str) -> Result<Headers>;
}
impl HeadersWithSet for Headers {
    fn with_set(self, name: &str, value: &str) -> Result<Headers> {
        self.set(name, value)?;
        Ok(self)
    }
}
