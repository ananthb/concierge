//! `GET /api/bootstrap` — everything the SPA needs to render its first frame.
//!
//! One request instead of four. The landing page needs pricing (for the
//! plan copy and the credit slider), the demo config and its persona
//! catalog (for the interactive chat), and whoever is signed in (so it can
//! route straight to the wizard or the dashboard instead of flashing the
//! marketing page first).
//!
//! Public: no session required. `session` is `null` when nobody is signed
//! in, which is the normal case for this endpoint.

use serde::Serialize;
use worker::*;

use crate::handlers::demo_personas_list::{self, DemoPersonasResponse};
use crate::locale::{Currency, Locale};
use crate::storage;

#[derive(Serialize)]
struct Bootstrap {
    pricing: PricingView,
    demo: DemoView,
    /// `None` for a signed-out visitor.
    session: Option<super::account::SessionView>,
    /// Resolved from `Accept-Language` + `CF-IPCountry` for a signed-out
    /// visitor, or from the tenant record once signed in.
    locale: LocaleView,
}

#[derive(Serialize)]
struct LocaleView {
    tag: String,
    currency: String,
}

#[derive(Serialize)]
struct PricingView {
    /// Per-AI-reply rate in milli-minor units (e.g. 10000 = ₹0.10), for
    /// the caller's resolved currency.
    unit_price_milli: i64,
    currency: String,
    min_credits: i64,
    max_credits: i64,
    /// Every currency we can quote in, so the pricing page's toggle
    /// doesn't need a second round-trip.
    currencies: Vec<CurrencyRate>,
}

#[derive(Serialize)]
struct CurrencyRate {
    code: String,
    unit_price_milli: i64,
}

#[derive(Serialize)]
struct DemoView {
    enabled: bool,
    max_user_turns: u32,
    idle_timeout_secs: u32,
    /// Empty when the demo is disabled. Resolved server-side (cache →
    /// cold-miss regeneration → empty) exactly as the old inline
    /// `<script type="application/json">` block did: the client never
    /// generates or fetches personas itself.
    personas: Vec<serde_json::Value>,
}

pub async fn get(req: Request, env: Env) -> Result<Response> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    // Signed-in state first: it decides which locale and currency the rest
    // of the payload is quoted in.
    let session = super::account::resolve_session(&req, &env).await?;
    let locale = match session.as_ref() {
        Some(s) => Locale::from_tenant(&s.locale, Some(Currency::parse(&s.currency))),
        None => Locale::from_request(&req),
    };

    let cfg = storage::get_pricing(&db).await;
    let code = locale.currency.as_str();

    let mut currencies: Vec<CurrencyRate> = [Currency::Inr, Currency::Usd]
        .iter()
        .map(|c| CurrencyRate {
            code: c.as_str().to_string(),
            unit_price_milli: cfg.unit_price_milli(c.as_str()),
        })
        .filter(|r| r.unit_price_milli > 0)
        .collect();
    currencies.sort_by(|a, b| a.code.cmp(&b.code));

    let demo_cfg = storage::get_demo_config(&kv).await.unwrap_or_default();
    let personas = if demo_cfg.enabled {
        let raw = demo_personas_list::resolve_personas_json(&env).await;
        serde_json::from_str::<DemoPersonasResponse>(&raw)
            .map(|r| {
                r.personas
                    .into_iter()
                    .filter_map(|p| serde_json::to_value(p).ok())
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    super::json(&Bootstrap {
        pricing: PricingView {
            unit_price_milli: cfg.unit_price_milli(code),
            currency: code.to_string(),
            min_credits: cfg.min_credits,
            max_credits: cfg.max_credits,
            currencies,
        },
        demo: DemoView {
            enabled: demo_cfg.enabled,
            max_user_turns: demo_cfg.max_user_turns,
            idle_timeout_secs: demo_cfg.idle_timeout_secs,
            personas,
        },
        locale: LocaleView {
            tag: locale.langid.to_string(),
            currency: code.to_string(),
        },
        session,
    })
}
