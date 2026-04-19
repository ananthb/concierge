//! Onboarding wizard handler — /admin/wizard/* routes

use worker::*;

use crate::storage::*;
use crate::templates::onboarding::*;
use crate::types::*;

use super::get_base_url;

pub async fn handle_wizard(
    mut req: Request,
    env: Env,
    path: &str,
    _method: Method,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let kv = env.kv("CALENDARS_KV")?;
    let mut state = get_onboarding(&kv, tenant_id).await?;

    let sub = path
        .strip_prefix("/admin/wizard")
        .unwrap_or("")
        .trim_start_matches('/');

    match sub {
        // Navigation between steps
        "goto" => {
            let form: serde_json::Value = req.json().await?;
            let to = form.get("to").and_then(|v| v.as_str()).unwrap_or("welcome");

            // Save biz name if coming from welcome
            if let Some(biz) = form.get("biz").and_then(|v| v.as_str()) {
                if !biz.is_empty() {
                    state.biz_name = biz.to_string();
                }
            }

            state.step = to.to_string();
            save_onboarding(&kv, tenant_id, &state).await?;

            render_step(to, &state, &kv, tenant_id, base_url).await
        }

        // Admin channel selection
        "admin-pick" => {
            let form: serde_json::Value = req.json().await?;
            let v = form.get("v").and_then(|v| v.as_str()).unwrap_or("");
            state.admin_channel = v.to_string();
            save_onboarding(&kv, tenant_id, &state).await?;

            Response::from_html(admin_pick_html(&state.admin_channel, base_url))
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
            state.step = "test".to_string();
            save_onboarding(&kv, tenant_id, &state).await?;

            Response::from_html(test_html(base_url))
        }

        // Default: show current step
        _ => {
            let step = if state.step.is_empty() {
                "welcome"
            } else {
                &state.step
            };
            render_step(step, &state, &kv, tenant_id, base_url).await
        }
    }
}

async fn render_step(
    step: &str,
    state: &OnboardingState,
    kv: &kv::KvStore,
    tenant_id: &str,
    base_url: &str,
) -> Result<Response> {
    match step {
        "welcome" => Response::from_html(welcome_html(base_url)),
        "connect" => {
            let wa = list_whatsapp_accounts(kv, tenant_id).await?;
            let ig = list_instagram_accounts(kv, tenant_id).await?;
            Response::from_html(connect_html(!ig.is_empty(), !wa.is_empty(), base_url))
        }
        "admin" => Response::from_html(admin_pick_html(&state.admin_channel, base_url)),
        "persona" => Response::from_html(persona_html(&state.persona, base_url)),
        "replies" => Response::from_html(replies_html(&state.canned_replies, base_url)),
        "test" => Response::from_html(test_html(base_url)),
        _ => Response::from_html(welcome_html(base_url)),
    }
}
