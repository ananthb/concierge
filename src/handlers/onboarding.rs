//! Onboarding wizard handler — /admin/wizard/* routes

use worker::*;

use crate::storage::*;
use crate::templates::onboarding::*;
use crate::types::*;

pub async fn handle_wizard(
    mut req: Request,
    env: Env,
    path: &str,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let kv = env.kv("KV")?;
    let mut state = get_onboarding(&kv, tenant_id).await?;

    let sub = path
        .strip_prefix("/admin/wizard")
        .unwrap_or("")
        .trim_start_matches('/');

    match sub {
        // Navigation between steps
        "goto" => {
            let form: serde_json::Value = req.json().await?;
            let to = form.get("to").and_then(|v| v.as_str()).unwrap_or("basics");

            // Auto-save persona fields if present
            if let Some(v) = form.get("biz_type").and_then(|v| v.as_str()) {
                if !v.is_empty() {
                    state.persona.biz_type = v.to_string();
                }
            }
            if let Some(v) = form.get("city").and_then(|v| v.as_str()) {
                if !v.is_empty() {
                    state.persona.city = v.to_string();
                }
            }
            if let Some(v) = form.get("tone").and_then(|v| v.as_str()) {
                if !v.is_empty() {
                    state.persona.tone = v.to_string();
                }
            }
            if let Some(v) = form.get("never").and_then(|v| v.as_str()) {
                if !v.is_empty() {
                    state.persona.never = v.to_string();
                }
            }

            state.step = to.to_string();
            save_onboarding(&kv, tenant_id, &state).await?;

            render_step(to, &state, &kv, &env, tenant_id, base_url).await
        }

        // Save business info (The basics step)
        "basics" => {
            let form: serde_json::Value = req.json().await?;
            let get = |key: &str| -> String {
                form.get(key)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string()
            };
            state.business.name = get("name");
            state.business.contact_name = get("contact_name");
            state.business.phone = get("phone");
            state.business.business_type = get("business_type");
            state.business.pan = get("pan").to_uppercase();
            state.business.gstin = get("gstin").to_uppercase();
            state.business.address = get("address");
            state.business.state = get("state");
            state.business.pincode = get("pincode");
            state.step = "channels".to_string();
            save_onboarding(&kv, tenant_id, &state).await?;

            render_step("channels", &state, &kv, &env, tenant_id, base_url).await
        }

        // Add email subdomain in wizard (no billing yet — deferred to Ship it)
        "email/add" => {
            let form: serde_json::Value = req.json().await?;
            let label = form
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();

            let base_domain = env
                .var("EMAIL_BASE_DOMAIN")
                .map(|v| v.to_string())
                .unwrap_or_default();

            if !base_domain.is_empty() && !label.is_empty() {
                if crate::cloudflare::dns::validate_subdomain_label(&label).is_ok() {
                    let domain = format!("{label}.{base_domain}");
                    let mut subs = get_email_subdomains(&kv, tenant_id).await?;

                    // Check uniqueness
                    if !subs.iter().any(|s| s.domain == domain)
                        && get_tenant_by_domain(&kv, &domain).await?.is_none()
                    {
                        subs.push(EmailSubdomain {
                            label: label.clone(),
                            domain,
                            tenant_id: tenant_id.to_string(),
                            default_action: EmailAction::Drop,
                            dns_record_ids: vec![],
                            subscription_id: None,
                            status: SubdomainStatus::Suspended,
                            created_at: crate::helpers::now_iso(),
                            updated_at: crate::helpers::now_iso(),
                        });
                        save_email_subdomains(&kv, tenant_id, &subs).await?;
                    }
                }
            }

            render_step("channels", &state, &kv, &env, tenant_id, base_url).await
        }

        // Remove email subdomain (only if not yet subscribed)
        "email/remove" => {
            let form: serde_json::Value = req.json().await?;
            let label = form
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();

            let mut subs = get_email_subdomains(&kv, tenant_id).await?;
            subs.retain(|s| !(s.label == label && s.subscription_id.is_none()));
            save_email_subdomains(&kv, tenant_id, &subs).await?;

            render_step("channels", &state, &kv, &env, tenant_id, base_url).await
        }

        // Save notification preferences
        "notifications" => {
            let form: serde_json::Value = req.json().await?;
            let is_true =
                |key: &str| -> bool { form.get(key).and_then(|v| v.as_str()) == Some("true") };
            let parse_freq = |key: &str, default: u32| -> u32 {
                form.get(key)
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u32>().ok())
                    .map(|v| v.clamp(5, 1440))
                    .unwrap_or(default)
            };

            state.notifications = NotificationConfig {
                approval_discord: is_true("approval_discord"),
                approval_email: is_true("approval_email"),
                approval_email_frequency_minutes: parse_freq("approval_freq", 60),
                digest_discord: is_true("digest_discord"),
                digest_email: is_true("digest_email"),
                digest_email_frequency_minutes: parse_freq("digest_freq", 1440),
            };
            state.step = "persona".to_string();
            save_onboarding(&kv, tenant_id, &state).await?;

            render_step("persona", &state, &kv, &env, tenant_id, base_url).await
        }

        // Persona save
        "persona" => {
            let form: serde_json::Value = req.json().await?;
            if let Some(v) = form.get("biz_type").and_then(|v| v.as_str()) {
                state.persona.biz_type = v.to_string();
            }
            if let Some(v) = form.get("city").and_then(|v| v.as_str()) {
                state.persona.city = v.to_string();
            }
            if let Some(v) = form.get("tone").and_then(|v| v.as_str()) {
                state.persona.tone = v.to_string();
            }
            if let Some(v) = form.get("never").and_then(|v| v.as_str()) {
                state.persona.never = v.to_string();
            }
            state.step = "replies".to_string();
            save_onboarding(&kv, tenant_id, &state).await?;

            Response::from_html(replies_html(&state.canned_replies, base_url))
        }

        // Add canned reply
        "replies/add" => {
            state.canned_replies.push(CannedReply {
                trigger: String::new(),
                reply: String::new(),
            });
            save_onboarding(&kv, tenant_id, &state).await?;
            Response::from_html(replies_html(&state.canned_replies, base_url))
        }

        // Delete canned reply
        "replies/del" => {
            let form: serde_json::Value = req.json().await?;
            let i = form
                .get("i")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            if i < state.canned_replies.len() {
                state.canned_replies.remove(i);
            }
            save_onboarding(&kv, tenant_id, &state).await?;
            Response::from_html(replies_html(&state.canned_replies, base_url))
        }

        // Save canned replies and go to test
        "replies/save" => {
            // Parse the form — triggers and replies come as trigger_0, reply_0, etc.
            let form: serde_json::Value = req.json().await?;
            let mut replies = Vec::new();
            let mut i = 0;
            loop {
                let trigger_key = format!("trigger_{i}");
                let reply_key = format!("reply_{i}");
                match (
                    form.get(&trigger_key).and_then(|v| v.as_str()),
                    form.get(&reply_key).and_then(|v| v.as_str()),
                ) {
                    (Some(trigger), Some(reply)) => {
                        if !trigger.is_empty() || !reply.is_empty() {
                            replies.push(CannedReply {
                                trigger: trigger.to_string(),
                                reply: reply.to_string(),
                            });
                        }
                    }
                    _ => break,
                }
                i += 1;
            }
            state.canned_replies = replies;
            state.step = "launch".to_string();
            save_onboarding(&kv, tenant_id, &state).await?;

            render_step("launch", &state, &kv, &env, tenant_id, base_url).await
        }

        // Complete the wizard
        "complete" => {
            state.completed = true;
            state.step = "launch".to_string();
            save_onboarding(&kv, tenant_id, &state).await?;

            let headers = Headers::new();
            headers.set("Location", &format!("{base_url}/admin"))?;
            Ok(Response::empty()?.with_status(302).with_headers(headers))
        }

        // Default: show current step (resume from where user left off)
        _ => {
            let step =
                if state.step.is_empty() || state.step == "welcome" || state.step == "business" {
                    // New users or legacy steps → start at "basics"
                    state.step = "basics".to_string();
                    let _ = save_onboarding(&kv, tenant_id, &state).await;
                    "basics"
                } else {
                    &state.step
                };
            render_step(step, &state, &kv, &env, tenant_id, base_url).await
        }
    }
}

async fn render_step(
    step: &str,
    state: &OnboardingState,
    kv: &kv::KvStore,
    env: &Env,
    tenant_id: &str,
    base_url: &str,
) -> Result<Response> {
    match step {
        "basics" => Response::from_html(basics_html(&state.business, base_url)),
        "channels" => {
            let wa = list_whatsapp_accounts(kv, tenant_id).await?;
            let ig = list_instagram_accounts(kv, tenant_id).await?;
            let email_subs = get_email_subdomains(kv, tenant_id).await?;
            let slug = crate::helpers::generate_slug().unwrap_or_else(|_| "my-biz".into());
            let base_domain = std::env::var("EMAIL_BASE_DOMAIN").unwrap_or_default();
            Response::from_html(connect_html(
                !ig.is_empty(),
                !wa.is_empty(),
                &email_subs,
                &slug,
                &base_domain,
                base_url,
            ))
        }
        "notifications" => Response::from_html(notifications_html(&state.notifications, base_url)),
        "persona" => Response::from_html(persona_html(&state.persona, base_url)),
        "replies" => Response::from_html(replies_html(&state.canned_replies, base_url)),
        "launch" => {
            let email_subs = get_email_subdomains(kv, tenant_id).await?;
            let db = env.d1("DB")?;
            let packs = crate::storage::get_active_credit_packs(&db)
                .await
                .unwrap_or_default();
            Response::from_html(launch_html(&email_subs, &packs, base_url))
        }
        _ => Response::from_html(basics_html(&state.business, base_url)),
    }
}
