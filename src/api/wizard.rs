//! `/api/wizard/*` — 3-step onboarding wizard: `basics` -> `channels` -> `launch`.

use serde::Serialize;
use worker::*;

use crate::billing;
use crate::helpers::sha256_hex;
use crate::safety;
use crate::storage::*;
use crate::types::*;

#[derive(Serialize)]
struct WizardView {
    step: String,
    steps: Vec<String>,
    completed: bool,
    business: BusinessInfo,
    profile_safety_status: String,
    whatsapp: Vec<super::whatsapp::AccountView>,
    signup: Option<super::whatsapp::SignupView>,
    launch: LaunchView,
}

#[derive(Serialize)]
struct LaunchView {
    balance: i64,
    unit_price_milli: i64,
    currency: String,
    min_credits: i64,
    max_credits: i64,
    metered: bool,
}

pub async fn handle(mut req: Request, env: Env, rest: &[&str]) -> Result<Response> {
    let tenant_id = match super::require_tenant(&req, &env).await {
        Ok(t) => t,
        Err(r) => return r,
    };
    let db = env.d1("DB")?;
    let method = req.method();
    let mut state = get_onboarding(&db, &tenant_id).await?;

    if state.completed && method != Method::Get {
        return super::error(
            "already_completed",
            "Setup is already finished. Change these from your dashboard.",
            409,
        );
    }

    match (method, rest) {
        (Method::Get, []) => view(&env, &db, &tenant_id, &state).await,

        (Method::Put, ["basics"]) => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            let name = super::field(&body, "name");
            let phone = super::field(&body, "phone");
            let business_type = super::field(&body, "business_type");
            if name.is_empty() {
                return super::invalid("Your brand name is required.");
            }
            if business_type.is_empty() {
                return super::invalid("Pick the entity type you're registered as.");
            }

            let old_business_name = state.business.name.clone();
            let content_hash = sha256_hex(&name);
            let verdict = safety::classify_text(&env, &name).await;
            state.business = BusinessInfo {
                name,
                contact_name: super::field(&body, "contact_name"),
                phone,
                business_type,
                pan: super::field(&body, "pan").to_uppercase(),
                gstin: super::field(&body, "gstin").to_uppercase(),
                address: super::field(&body, "address"),
                state: super::field(&body, "state"),
                pincode: super::field(&body, "pincode"),
            };
            state.profile_safety = safety::verdict_to_state(verdict, content_hash);

            state.step = advance(state.step, OnboardingStep::Channels);
            save_onboarding(&db, &tenant_id, &state).await?;

            let old_default = crate::prompt::default_bot_prompt_for(&old_business_name);
            let new_default = crate::prompt::default_bot_prompt_for(&state.business.name);
            if new_default != old_default || old_business_name.trim().is_empty() {
                let accounts = list_whatsapp_accounts(&db, &tenant_id).await?;
                for mut account in accounts {
                    if (account.auto_reply.bot_prompt == crate::prompt::DEFAULT_BOT_PROMPT
                        || account.auto_reply.bot_prompt == old_default)
                        && account.auto_reply.bot_prompt != new_default
                    {
                        account.auto_reply.bot_prompt = new_default.clone();
                        account.canned_safety = SafetyState::default();
                        account.updated_at = crate::helpers::now_iso();
                        save_whatsapp_account(&db, &account).await?;
                    }
                }
            }

            view(&env, &db, &tenant_id, &state).await
        }

        (Method::Post, ["channels", "done"]) => {
            let has_wa = !list_whatsapp_accounts(&db, &tenant_id).await?.is_empty();
            if !has_wa {
                return super::invalid("Connect a WhatsApp Business number to continue.");
            }
            state.step = advance(state.step, OnboardingStep::Launch);
            save_onboarding(&db, &tenant_id, &state).await?;
            view(&env, &db, &tenant_id, &state).await
        }

        (Method::Post, ["step"]) => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            let Some(to) = OnboardingStep::from_wire(&super::field(&body, "to")) else {
                return super::invalid("Unknown step.");
            };
            if to.index() > state.step.index() {
                return super::invalid("Finish this step first.");
            }
            state.step = to;
            save_onboarding(&db, &tenant_id, &state).await?;
            view(&env, &db, &tenant_id, &state).await
        }

        (Method::Post, ["complete"]) => {
            if state.business.name.trim().is_empty() {
                return super::invalid("Complete your business basics before finishing.");
            }
            let has_wa = !list_whatsapp_accounts(&db, &tenant_id).await?.is_empty();
            if !has_wa {
                return super::invalid("Connect a WhatsApp Business number before finishing.");
            }

            state.completed = true;
            state.step = OnboardingStep::Launch;
            save_onboarding(&db, &tenant_id, &state).await?;
            view(&env, &db, &tenant_id, &state).await
        }

        _ => super::not_found("Endpoint"),
    }
}

fn advance(current: OnboardingStep, target: OnboardingStep) -> OnboardingStep {
    if current.index() > target.index() {
        current
    } else {
        target
    }
}

async fn view(
    env: &Env,
    db: &D1Database,
    tenant_id: &str,
    state: &OnboardingState,
) -> Result<Response> {
    let accounts = list_whatsapp_accounts(db, tenant_id).await?;
    let tenant = get_tenant(db, tenant_id).await?.unwrap_or_default();
    let mut bill = get_tenant_billing(db, tenant_id).await?;
    billing::refresh_billing(&mut bill);

    let locale = crate::locale::Locale::from_tenant(&tenant.locale, Some(tenant.currency));
    let code = locale.currency.as_str();

    super::json(&WizardView {
        step: state.step.as_str().to_string(),
        steps: OnboardingStep::ALL
            .iter()
            .map(|s| s.as_str().to_string())
            .collect(),
        completed: state.completed,
        business: state.business.clone(),
        profile_safety_status: state.profile_safety.status.as_str().to_string(),
        whatsapp: accounts
            .iter()
            .map(super::whatsapp::AccountView::from)
            .collect(),
        signup: super::whatsapp::SignupView::resolve(env, db, tenant_id).await?,
        launch: LaunchView {
            balance: bill.total_remaining(),
            unit_price_milli: billing::unit_price_milli(code),
            currency: code.to_string(),
            min_credits: billing::MIN_CREDITS,
            max_credits: billing::MAX_CREDITS,
            metered: tenant.plan.is_metered(),
        },
    })
}
