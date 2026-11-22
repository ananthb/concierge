//! `GET /api/bootstrap` — everything the SPA needs to render its first frame.

use serde::Serialize;
use worker::*;

use crate::api::demo::{DemoFixture, DEMO_IDLE_TIMEOUT_SECS, DEMO_MAX_USER_TURNS, FIXTURES};
use crate::billing;
use crate::locale::{Currency, Locale};

#[derive(Serialize)]
struct Bootstrap {
    pricing: PricingView,
    demo: DemoView,
    session: Option<super::account::SessionView>,
    locale: LocaleView,
}

#[derive(Serialize)]
struct LocaleView {
    tag: String,
    currency: String,
}

#[derive(Serialize)]
struct PricingView {
    unit_price_milli: i64,
    currency: String,
    min_credits: i64,
    max_credits: i64,
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
    personas: &'static [DemoFixture],
}

pub async fn get(req: Request, env: Env) -> Result<Response> {
    let session = super::account::resolve_session(&req, &env).await?;
    let locale = match session.as_ref() {
        Some(s) => Locale::from_tenant(&s.locale, Some(Currency::parse(&s.currency))),
        None => Locale::from_request(&req),
    };

    let code = locale.currency.as_str();
    let mut currencies: Vec<CurrencyRate> = [Currency::Inr, Currency::Usd]
        .iter()
        .map(|c| CurrencyRate {
            code: c.as_str().to_string(),
            unit_price_milli: billing::unit_price_milli(c.as_str()),
        })
        .filter(|r| r.unit_price_milli > 0)
        .collect();
    currencies.sort_by(|a, b| a.code.cmp(&b.code));

    super::json(&Bootstrap {
        pricing: PricingView {
            unit_price_milli: billing::unit_price_milli(code),
            currency: code.to_string(),
            min_credits: billing::MIN_CREDITS,
            max_credits: billing::MAX_CREDITS,
            currencies,
        },
        demo: DemoView {
            enabled: true,
            max_user_turns: DEMO_MAX_USER_TURNS,
            idle_timeout_secs: DEMO_IDLE_TIMEOUT_SECS,
            personas: FIXTURES,
        },
        locale: LocaleView {
            tag: locale.langid.to_string(),
            currency: code.to_string(),
        },
        session,
    })
}
