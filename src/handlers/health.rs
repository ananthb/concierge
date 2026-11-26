//! Health / readiness checks (`/health`).

use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Warn,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

#[derive(Serialize)]
struct PublicHealth {
    overall: Status,
    generated_at: String,
}

pub trait Component {
    fn name(&self) -> &'static str;
    fn requirements(&self) -> &'static [Requirement];

    fn required(&self) -> bool {
        true
    }

    fn shallow_check(&self, env: &Env) -> Check {
        let missing: Vec<&'static str> = self
            .requirements()
            .iter()
            .filter(|r| !r.satisfied(env))
            .map(|r| r.name())
            .collect();
        let total = self.requirements().len();
        let status = classify(self.required(), missing.len(), total);
        let detail = match status {
            Status::Ok if total == 1 => "configured".to_string(),
            Status::Ok => format!("{total} requirements set"),
            Status::Warn => "not configured".to_string(),
            Status::Error => format!("missing: {}", missing.join(", ")),
        };
        Check {
            name: self.name().into(),
            status,
            detail,
        }
    }
}

fn classify(required: bool, missing: usize, total: usize) -> Status {
    if missing == 0 {
        Status::Ok
    } else if !required && missing == total {
        Status::Warn
    } else {
        Status::Error
    }
}

// ============================================================================
// Concrete components
// ============================================================================

pub struct WhatsAppSignup;
impl Component for WhatsAppSignup {
    fn name(&self) -> &'static str {
        "WhatsApp signup"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[
            Requirement::Var("META_APP_ID"),
            Requirement::Secret("META_APP_SECRET"),
            Requirement::Secret("WHATSAPP_ACCESS_TOKEN"),
            Requirement::Var("WHATSAPP_SIGNUP_CONFIG_ID"),
        ]
    }
    fn required(&self) -> bool {
        false
    }
}

pub struct Razorpay;
impl Component for Razorpay {
    fn name(&self) -> &'static str {
        "Razorpay"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[
            Requirement::Var("RAZORPAY_KEY_ID"),
            Requirement::Secret("RAZORPAY_KEY_SECRET"),
            Requirement::Secret("RAZORPAY_WEBHOOK_SECRET"),
        ]
    }
    fn required(&self) -> bool {
        false
    }
}

pub struct AiBinding;
impl Component for AiBinding {
    fn name(&self) -> &'static str {
        "AI binding"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[Requirement::Binding("AI")]
    }
}

/// Outbound handoff notification email (`[[send_email]]` + `EMAIL_DOMAIN`).
/// Optional: missing `EMAIL_DOMAIN` never fails `/health` 503.
pub struct EmailBinding;
impl Component for EmailBinding {
    fn name(&self) -> &'static str {
        "EMAIL handoff alerts"
    }
    fn requirements(&self) -> &'static [Requirement] {
        &[
            Requirement::Binding("EMAIL"),
            Requirement::Var("EMAIL_DOMAIN"),
        ]
    }
    fn required(&self) -> bool {
        false
    }
    fn shallow_check(&self, env: &Env) -> Check {
        // If EMAIL_DOMAIN is unset, outbound handoff emails are intentionally
        // disabled — report Warn (not Error) even when the wrangler EMAIL
        // binding is present.
        if !var_set(env, "EMAIL_DOMAIN") {
            return Check {
                name: self.name().into(),
                status: Status::Warn,
                detail: "EMAIL_DOMAIN not set (handoff alerts disabled)".into(),
            };
        }
        if !binding_present(env, "EMAIL") {
            return Check {
                name: self.name().into(),
                status: Status::Warn,
                detail: "EMAIL binding missing".into(),
            };
        }
        Check {
            name: self.name().into(),
            status: Status::Ok,
            detail: "configured".into(),
        }
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
}
impl D1Binding {
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

// ============================================================================
// Registry + public API
// ============================================================================

const SHALLOW_COMPONENTS: &[&dyn Component] =
    &[&AiBinding, &EmailBinding, &WhatsAppSignup, &Razorpay];

pub fn essentials_missing(env: &Env) -> Vec<&'static str> {
    let _ = env;
    Vec::new()
}

pub async fn handle_health(_req: Request, env: Env) -> Result<Response> {
    let mut checks = Vec::new();
    checks.push(D1Binding::probe(&env).await);
    for c in SHALLOW_COMPONENTS {
        checks.push(c.shallow_check(&env));
    }

    let overall = rollup(&checks);
    let public = PublicHealth {
        overall,
        generated_at: crate::helpers::now_iso(),
    };
    let body = serde_json::to_string(&public)?;
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Cache-Control", "no-store")?;
    let status = match overall {
        Status::Ok | Status::Warn => 200,
        Status::Error => 503,
    };
    Ok(Response::ok(body)?
        .with_status(status)
        .with_headers(headers))
}

fn rollup(checks: &[Check]) -> Status {
    checks
        .iter()
        .map(|c| c.status)
        .fold(Status::Ok, |acc, s| match (acc, s) {
            (Status::Error, _) | (_, Status::Error) => Status::Error,
            (Status::Warn, _) | (_, Status::Warn) => Status::Warn,
            _ => Status::Ok,
        })
}

fn secret_set(env: &Env, name: &str) -> bool {
    env.secret(name)
        .map(|s| !s.to_string().trim().is_empty())
        .unwrap_or(false)
}

fn var_set(env: &Env, name: &str) -> bool {
    env.var(name)
        .map(|v| !v.to_string().trim().is_empty())
        .unwrap_or(false)
}

fn binding_present(env: &Env, name: &str) -> bool {
    use wasm_bindgen::JsValue;
    let env_js: JsValue = env.clone().into();
    js_sys::Reflect::get(&env_js, &JsValue::from_str(name))
        .map(|v| !v.is_undefined())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(name: &str, status: Status) -> Check {
        Check {
            name: name.into(),
            status,
            detail: String::new(),
        }
    }

    #[test]
    fn classify_tiers() {
        assert_eq!(classify(true, 0, 3), Status::Ok);
        assert_eq!(classify(false, 0, 3), Status::Ok);
        assert_eq!(classify(false, 3, 3), Status::Warn);
        assert_eq!(classify(true, 1, 1), Status::Error);
        assert_eq!(classify(false, 1, 3), Status::Error);
    }

    #[test]
    fn rollup_ignores_warnings_but_not_errors() {
        assert_eq!(
            rollup(&[
                check("D1", Status::Ok),
                check("AI", Status::Ok),
                check("Razorpay", Status::Warn),
                check("EMAIL handoff alerts", Status::Warn),
            ]),
            Status::Warn
        );
        assert_eq!(
            rollup(&[check("AI", Status::Ok), check("D1", Status::Error)]),
            Status::Error
        );
    }

    #[test]
    fn required_vs_optional_components() {
        assert!(!WhatsAppSignup.required());
        assert!(!Razorpay.required());
        assert!(!EmailBinding.required());

        assert!(AiBinding.required());
        assert!(D1Binding.required());

        assert_eq!(
            WhatsAppSignup.requirements(),
            &[
                Requirement::Var("META_APP_ID"),
                Requirement::Secret("META_APP_SECRET"),
                Requirement::Secret("WHATSAPP_ACCESS_TOKEN"),
                Requirement::Var("WHATSAPP_SIGNUP_CONFIG_ID"),
            ]
        );
        assert_eq!(
            Razorpay.requirements(),
            &[
                Requirement::Var("RAZORPAY_KEY_ID"),
                Requirement::Secret("RAZORPAY_KEY_SECRET"),
                Requirement::Secret("RAZORPAY_WEBHOOK_SECRET"),
            ]
        );
    }
}
