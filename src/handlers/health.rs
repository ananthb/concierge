//! Health / connection-status checks.
//!
//! `/health` returns a JSON `HealthReport`. The `/manage` dashboard renders
//! the same report as HTML (the management module owns that view).
//!
//! Two depths:
//! - **shallow**: only checks bindings (D1, KV, Email, AI, DO) and which
//!   secrets are configured. Cheap, runs on every hit.
//! - **deep**  : additionally pings external APIs (Discord, Cloudflare DNS).
//!   Cached in KV for 60s so /manage doesn't hammer providers.

use serde::{Deserialize, Serialize};
use worker::*;

const DEEP_CACHE_KEY: &str = "health:deep:cache";
const DEEP_CACHE_TTL_SECS: u64 = 60;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Warn,
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Check {
    pub name: String,
    pub status: Status,
    pub detail: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HealthReport {
    pub overall: Status,
    pub generated_at: String,
    pub deep: bool,
    pub checks: Vec<Check>,
}

/// Public health endpoint. Returns ONLY the rollup: no per-check detail,
/// no secret names. Detailed status lives on /manage (Cloudflare Access
/// protected). Operators who need more from a public endpoint can lock
/// /health down further with Access on their own zone.
#[derive(Serialize)]
struct PublicHealth {
    overall: Status,
    generated_at: String,
}

/// Quick synchronous check for the secrets that are *required* to serve any
/// user-facing flow at all (sessions and login). When any of these are
/// missing the worker can't actually do anything useful: we surface a
/// maintenance page rather than letting the user reach a broken OAuth
/// redirect or a session that can't be encrypted.
///
/// The shallow `secrets_check` in `run_checks()` is more comprehensive but
/// also flags optional integrations as `Error` when missing, which is too
/// aggressive for the "are we open for business" question.
pub fn essentials_missing(env: &Env) -> Vec<&'static str> {
    let required: &[&str] = &[
        "ENCRYPTION_KEY",
        "GOOGLE_OAUTH_CLIENT_ID",
        "GOOGLE_OAUTH_CLIENT_SECRET",
    ];
    required
        .iter()
        .filter(|name| {
            env.secret(name)
                .map(|s| s.to_string().is_empty())
                .unwrap_or(true)
        })
        .copied()
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

/// Build a HealthReport. `deep=true` pings external providers (cached 60s).
pub async fn run_checks(env: &Env, deep: bool) -> HealthReport {
    let mut checks = Vec::new();

    // ---- Bindings (always shallow) -----------------------------------
    checks.push(check_d1(env).await);
    checks.push(check_kv(env).await);
    checks.push(binding_check("AI", env.ai("AI").is_ok()));
    checks.push(binding_check(
        "REPLY_BUFFER",
        env.durable_object("REPLY_BUFFER").is_ok(),
    ));
    // EMAIL is a send-binding, no Rust accessor; check via env JsValue.
    checks.push(binding_check_named_env(env, "EMAIL"));

    // ---- Secrets (shallow) -------------------------------------------
    checks.push(secrets_check(
        "Discord",
        env,
        &[
            "DISCORD_APPLICATION_ID",
            "DISCORD_PUBLIC_KEY",
            "DISCORD_BOT_TOKEN",
        ],
    ));
    checks.push(secrets_check(
        "Meta (WhatsApp + Instagram)",
        env,
        &[
            "META_APP_ID",
            "META_APP_SECRET",
            "WHATSAPP_ACCESS_TOKEN",
            "WHATSAPP_VERIFY_TOKEN",
            "INSTAGRAM_VERIFY_TOKEN",
        ],
    ));
    checks.push(secrets_check(
        "Razorpay",
        env,
        &[
            "RAZORPAY_KEY_ID",
            "RAZORPAY_KEY_SECRET",
            "RAZORPAY_WEBHOOK_SECRET",
        ],
    ));
    checks.push(secrets_check(
        "Google OAuth",
        env,
        &["GOOGLE_OAUTH_CLIENT_ID", "GOOGLE_OAUTH_CLIENT_SECRET"],
    ));
    checks.push(secrets_check("Encryption key", env, &["ENCRYPTION_KEY"]));

    // ---- Deep checks (cached) ----------------------------------------
    if deep {
        let kv_ok = env.kv("KV").ok();
        if let Some(kv) = kv_ok.as_ref() {
            // Try the cache first.
            if let Ok(Some(cached)) = kv.get(DEEP_CACHE_KEY).text().await {
                if let Ok(report) = serde_json::from_str::<HealthReport>(&cached) {
                    return report;
                }
            }
        }
        checks.push(deep_discord(env).await);

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
// Individual checks
// ============================================================================

fn binding_check(name: &str, present: bool) -> Check {
    if present {
        Check {
            name: format!("{name} binding"),
            status: Status::Ok,
            detail: "configured".into(),
        }
    } else {
        Check {
            name: format!("{name} binding"),
            status: Status::Error,
            detail: "missing: check wrangler.toml".into(),
        }
    }
}

fn binding_check_named_env(env: &Env, name: &str) -> Check {
    use wasm_bindgen::JsValue;
    let env_js: JsValue = env.clone().into();
    let present = js_sys::Reflect::get(&env_js, &JsValue::from_str(name))
        .map(|v| !v.is_undefined())
        .unwrap_or(false);
    binding_check(name, present)
}

async fn check_d1(env: &Env) -> Check {
    let db = match env.d1("DB") {
        Ok(d) => d,
        Err(_) => {
            return Check {
                name: "D1 (DB binding)".into(),
                status: Status::Error,
                detail: "binding missing".into(),
            }
        }
    };
    match db
        .prepare("SELECT 1 as ok")
        .first::<serde_json::Value>(None)
        .await
    {
        Ok(_) => Check {
            name: "D1 (DB binding)".into(),
            status: Status::Ok,
            detail: "reachable".into(),
        },
        Err(e) => Check {
            name: "D1 (DB binding)".into(),
            status: Status::Error,
            detail: format!("query failed: {e}"),
        },
    }
}

async fn check_kv(env: &Env) -> Check {
    let kv = match env.kv("KV") {
        Ok(k) => k,
        Err(_) => {
            return Check {
                name: "KV (KV binding)".into(),
                status: Status::Error,
                detail: "binding missing".into(),
            }
        }
    };
    // A get on a likely-missing key returns None: that's still "reachable".
    match kv.get("__healthcheck").text().await {
        Ok(_) => Check {
            name: "KV (KV binding)".into(),
            status: Status::Ok,
            detail: "reachable".into(),
        },
        Err(e) => Check {
            name: "KV (KV binding)".into(),
            status: Status::Error,
            detail: format!("get failed: {e}"),
        },
    }
}

fn secrets_check(name: &str, env: &Env, keys: &[&str]) -> Check {
    let missing: Vec<&str> = keys
        .iter()
        .copied()
        .filter(|k| {
            env.secret(k)
                .map(|s| s.to_string())
                .map(|s| s.is_empty())
                .unwrap_or(true)
        })
        .collect();
    if missing.is_empty() {
        Check {
            name: name.into(),
            status: Status::Ok,
            detail: format!("{} secrets configured", keys.len()),
        }
    } else {
        Check {
            name: name.into(),
            status: Status::Error,
            detail: format!("missing: {}", missing.join(", ")),
        }
    }
}

async fn deep_discord(env: &Env) -> Check {
    let token = env
        .secret("DISCORD_BOT_TOKEN")
        .map(|s| s.to_string())
        .unwrap_or_default();
    if token.is_empty() {
        return Check {
            name: "Discord bot reachable".into(),
            status: Status::Warn,
            detail: "DISCORD_BOT_TOKEN not set, skipped".into(),
        };
    }
    let url = "https://discord.com/api/v10/users/@me";
    let headers = match Headers::new().with_set("Authorization", &format!("Bot {token}")) {
        Ok(h) => h,
        Err(_) => {
            return Check {
                name: "Discord bot reachable".into(),
                status: Status::Error,
                detail: "couldn't build auth header".into(),
            }
        }
    };
    let mut init = RequestInit::new();
    init.with_method(Method::Get).with_headers(headers);
    let req = match Request::new_with_init(url, &init) {
        Ok(r) => r,
        Err(e) => {
            return Check {
                name: "Discord bot reachable".into(),
                status: Status::Error,
                detail: format!("request build: {e}"),
            }
        }
    };
    match Fetch::Request(req).send().await {
        Ok(mut r) if r.status_code() == 200 => Check {
            name: "Discord bot reachable".into(),
            status: Status::Ok,
            detail: r
                .json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v.get("username").and_then(|s| s.as_str()).map(String::from))
                .map(|u| format!("authenticated as @{u}"))
                .unwrap_or_else(|| "200 OK".into()),
        },
        Ok(r) => Check {
            name: "Discord bot reachable".into(),
            status: Status::Error,
            detail: format!("Discord returned HTTP {}", r.status_code()),
        },
        Err(e) => Check {
            name: "Discord bot reachable".into(),
            status: Status::Error,
            detail: format!("fetch failed: {e}"),
        },
    }
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
