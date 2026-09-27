//! `/api/wizard/*` — the onboarding wizard.
//!
//! Four steps: business basics, connect WhatsApp, pick a persona, launch.
//! The server owns which step the tenant is on; the frontend renders the
//! step it's told about and cannot skip ahead, because every advance is a
//! write here that also validates the step being left behind.
//!
//! The wizard is one-shot. Once `completed` is set, every endpoint answers
//! `409 already_completed` — the same rule the HTML wizard enforced by
//! redirecting to `/dashboard`. Ongoing edits live on the dashboard.
//!
//! Steps dropped in the cut: the notifications step (Discord is gone and
//! the email digest has no cadence left to pick) and the per-channel reply
//! rules the persona step used to seed.

use serde::Serialize;
use worker::*;

use crate::storage::*;
use crate::types::*;

/// `GET /api/wizard` returns this; every mutation returns it again so the
/// frontend never has to re-fetch to learn the new step.
#[derive(Serialize)]
struct WizardView {
    step: String,
    /// All steps in display order, for the progress indicator.
    steps: Vec<String>,
    completed: bool,
    business: BusinessInfo,
    persona: PersonaView,
    /// Connected WhatsApp numbers. Empty means the channels step is unmet.
    whatsapp: Vec<super::whatsapp::AccountView>,
    /// Meta Embedded Signup parameters, or `None` when the operator hasn't
    /// configured WhatsApp signup on this deploy.
    signup: Option<super::whatsapp::SignupView>,
    launch: LaunchView,
}

#[derive(Serialize)]
struct PersonaView {
    archetype_slug: String,
    goal: String,
    goal_url: String,
    handoff_conditions: Vec<String>,
    safety_status: String,
}

#[derive(Serialize)]
struct LaunchView {
    /// True once a Razorpay payment has been captured for this tenant. The
    /// wizard refuses to finish until then on metered plans: it's the
    /// abuse gate for fresh sign-ups.
    verified: bool,
    /// Refundable verification charge, in minor units of `currency`.
    verification_amount: i64,
    unit_price_milli: i64,
    currency: String,
    min_credits: i64,
    max_credits: i64,
    /// Complimentary accounts skip verification and can't buy credits.
    metered: bool,
}

pub async fn handle(mut req: Request, env: Env, rest: &[&str]) -> Result<Response> {
    let tenant_id = match super::require_tenant(&req, &env).await {
        Ok(t) => t,
        Err(r) => return r,
    };
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;
    let method = req.method();
    let mut state = get_onboarding(&kv, &tenant_id).await?;

    // Reading a sealed wizard is fine — the frontend uses it to decide
    // where to send the user. Writing to one is not.
    if state.completed && method != Method::Get {
        return super::error(
            "already_completed",
            "Setup is already finished. Change these from your dashboard.",
            409,
        );
    }

    match (method, rest) {
        (Method::Get, []) => view(&env, &kv, &db, &tenant_id, &state).await,

        // Business basics. Advancing requires the three fields an invoice
        // can't be raised without.
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
            if phone.is_empty() {
                return super::invalid("A contact phone number is required.");
            }
            if business_type.is_empty() {
                return super::invalid("Pick the entity type you're registered as.");
            }

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
            state.step = advance(state.step, OnboardingStep::Channels);
            save_onboarding(&kv, &tenant_id, &state).await?;
            view(&env, &kv, &db, &tenant_id, &state).await
        }

        // Leave the channels step. Refuses while no number is connected:
        // WhatsApp is the product, so an account with no number attached
        // has nothing to launch.
        (Method::Post, ["channels", "done"]) => {
            let accounts = list_whatsapp_accounts(&kv, &tenant_id).await?;
            if accounts.is_empty() {
                return super::invalid("Connect a WhatsApp number to continue.");
            }
            state.step = advance(state.step, OnboardingStep::Persona);
            save_onboarding(&kv, &tenant_id, &state).await?;
            view(&env, &kv, &db, &tenant_id, &state).await
        }

        // Pick a persona archetype and answer the two questions tenants are
        // best placed to answer up front. The rest of the builder fields are
        // edited later from the dashboard.
        (Method::Put, ["persona"]) => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            let slug = super::field(&body, "archetype_slug");

            // An invalid slug falls back to the first approved archetype
            // rather than erroring: the catalog is operator-managed and can
            // change under a tenant mid-wizard.
            let archetype = match get_archetype(&db, &slug).await? {
                Some(a) => a,
                None => match list_archetypes(&db, true).await?.first().cloned() {
                    Some(a) => a,
                    None => {
                        return super::error(
                            "no_archetypes",
                            "No personas are available right now. Try again shortly.",
                            503,
                        )
                    }
                },
            };

            let goal: String = super::field(&body, "goal").chars().take(120).collect();
            let goal_url: String =
                crate::personas::sanitize_goal_url(&super::field(&body, "goal_url"))
                    .chars()
                    .take(200)
                    .collect();
            let handoff_conditions: Vec<String> = body
                .get("handoff_conditions")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.trim().chars().take(120).collect::<String>())
                        .filter(|s| !s.is_empty())
                        .take(5)
                        .collect()
                })
                .unwrap_or_default();

            state.persona = PersonaConfig {
                source: PersonaSource::Builder(PersonaBuilder {
                    archetype_slug: archetype.slug.clone(),
                    biz_name: state.business.name.clone(),
                    goal,
                    goal_url,
                    handoff_conditions,
                    ..Default::default()
                }),
                safety: PersonaSafety {
                    status: PersonaSafetyStatus::Pending,
                    ..Default::default()
                },
            };

            // Every persona change re-enters the safety queue. AI replies
            // stay blocked tenant-wide until the verdict comes back
            // Approved for this exact prompt hash.
            let job = crate::safety_queue::SafetyJob {
                target: crate::safety_queue::SafetyJobTarget::Tenant {
                    tenant_id: tenant_id.clone(),
                },
                prompt_hash: state.persona.active_prompt_hash(&archetype.voice_prompt),
            };

            state.step = advance(state.step, OnboardingStep::Launch);
            save_onboarding(&kv, &tenant_id, &state).await?;
            // After the save: a lost enqueue leaves the persona Pending,
            // which is the safe direction. A successful enqueue against an
            // unsaved persona would approve a hash nobody has.
            let _ = crate::safety_queue::enqueue(&env, job).await;
            view(&env, &kv, &db, &tenant_id, &state).await
        }

        // Step back. Forward jumps are refused: the step a tenant is on is
        // the furthest they've legitimately reached.
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
            save_onboarding(&kv, &tenant_id, &state).await?;
            view(&env, &kv, &db, &tenant_id, &state).await
        }

        // Seal the wizard.
        (Method::Post, ["complete"]) => {
            let tenant = get_tenant(&db, &tenant_id).await?.unwrap_or_default();
            if tenant.plan.is_metered() && tenant.verified_at.is_none() {
                return super::error(
                    "unverified",
                    "Verify your account before finishing setup.",
                    402,
                );
            }
            if list_whatsapp_accounts(&kv, &tenant_id).await?.is_empty() {
                return super::invalid("Connect a WhatsApp number before finishing.");
            }

            state.completed = true;
            state.step = OnboardingStep::Launch;
            save_onboarding(&kv, &tenant_id, &state).await?;
            view(&env, &kv, &db, &tenant_id, &state).await
        }

        _ => super::not_found("Endpoint"),
    }
}

/// Move forward to `target` without ever moving backwards.
///
/// A tenant editing an earlier step (having already reached Launch) should
/// stay at Launch rather than being dragged back through the flow.
fn advance(current: OnboardingStep, target: OnboardingStep) -> OnboardingStep {
    if current.index() > target.index() {
        current
    } else {
        target
    }
}

async fn view(
    env: &Env,
    kv: &kv::KvStore,
    db: &D1Database,
    tenant_id: &str,
    state: &OnboardingState,
) -> Result<Response> {
    let accounts = list_whatsapp_accounts(kv, tenant_id).await?;
    let tenant = get_tenant(db, tenant_id).await?.unwrap_or_default();
    let locale = crate::locale::Locale::from_tenant(&tenant.locale, Some(tenant.currency));
    let cfg = get_pricing(db).await;
    let code = locale.currency.as_str();

    let (archetype_slug, goal, goal_url, handoff_conditions) = match &state.persona.source {
        PersonaSource::Builder(b) => (
            b.archetype_slug.clone(),
            b.goal.clone(),
            b.goal_url.clone(),
            b.handoff_conditions.clone(),
        ),
        // A tenant who wrote a raw prompt on the dashboard then stepped back
        // into the wizard has no archetype to show. Empty is correct: the
        // picker starts unselected.
        PersonaSource::Custom(_) => (String::new(), String::new(), String::new(), Vec::new()),
    };

    super::json(&WizardView {
        step: state.step.as_str().to_string(),
        steps: OnboardingStep::ALL
            .iter()
            .map(|s| s.as_str().to_string())
            .collect(),
        completed: state.completed,
        business: state.business.clone(),
        persona: PersonaView {
            archetype_slug,
            goal,
            goal_url,
            handoff_conditions,
            safety_status: super::persona::safety_wire(&state.persona.safety.status).to_string(),
        },
        whatsapp: accounts
            .iter()
            .map(super::whatsapp::AccountView::from)
            .collect(),
        signup: super::whatsapp::SignupView::resolve(env, kv, tenant_id).await?,
        launch: LaunchView {
            verified: tenant.verified_at.is_some(),
            verification_amount: cfg.verification_amount(code),
            unit_price_milli: cfg.unit_price_milli(code),
            currency: code.to_string(),
            min_credits: cfg.min_credits,
            max_credits: cfg.max_credits,
            metered: tenant.plan.is_metered(),
        },
    })
}
