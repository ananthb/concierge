//! Onboarding wizard handler: /admin/wizard/* routes

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
    let db = env.d1("DB")?;
    let mut state = get_onboarding(&kv, tenant_id).await?;

    let sub = path
        .strip_prefix("/admin/wizard")
        .unwrap_or("")
        .trim_start_matches('/');

    // Direct step access via URL (GET): allow going back, block skipping ahead.
    // Must come before the form-submit arms below, which share step names with
    // POST handlers ("basics", "notifications") and would otherwise try to
    // parse a non-existent JSON body on GET.
    if req.method() == Method::Get
        && matches!(
            sub,
            "basics" | "channels" | "notifications" | "replies" | "launch"
        )
    {
        let requested_idx = step_index(sub);
        let current_idx = step_index(&state.step);
        if requested_idx <= current_idx {
            return render_step(sub, &state, &kv, &env, tenant_id, base_url).await;
        }
        let headers = Headers::new();
        headers.set(
            "Location",
            &format!("{base_url}/admin/wizard/{}", state.step),
        )?;
        return Ok(Response::empty()?.with_status(302).with_headers(headers));
    }

    match sub {
        // Navigation between steps
        "goto" => {
            let form: serde_json::Value = req.json().await?;
            let to = form.get("to").and_then(|v| v.as_str()).unwrap_or("basics");

            state.step = to.to_string();
            save_onboarding(&kv, tenant_id, &state).await?;

            // Redirect so the URL changes (enables back/forward)
            let headers = Headers::new();
            headers.set("HX-Redirect", &format!("{base_url}/admin/wizard/{to}"))?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
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
            let name = get("name");
            let phone = get("phone");

            if name.is_empty() || phone.is_empty() {
                return Response::from_html(
                    "<div class=\"error\">Brand name and phone are required.</div>".to_string(),
                );
            }

            state.business.name = name;
            state.business.contact_name = get("contact_name");
            state.business.phone = phone;
            state.business.business_type = get("business_type");
            state.business.pan = get("pan").to_uppercase();
            state.business.gstin = get("gstin").to_uppercase();
            state.business.address = get("address");
            state.business.state = get("state");
            state.business.pincode = get("pincode");
            state.step = "channels".to_string();
            save_onboarding(&kv, tenant_id, &state).await?;

            let headers = Headers::new();
            headers.set("HX-Redirect", &format!("{base_url}/admin/wizard/channels"))?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Add an email address in the wizard. No payment gate: every tenant
        // gets one free address; later additions go through Billing.
        "email/add" => {
            let form: serde_json::Value = req.json().await?;
            let label = form
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();

            if crate::email::validate_local_part(&label).is_ok() {
                let tenant = get_tenant(&db, tenant_id).await?.unwrap_or_default();
                let addrs = get_email_addresses(&kv, tenant_id).await?;
                let at_quota = (addrs.len() as u32) >= tenant.email_address_quota();
                let already_owned = addrs.iter().any(|a| a.local_part == label);
                let globally_taken = get_tenant_by_address(&kv, &label).await?.is_some();
                if !at_quota && !already_owned && !globally_taken {
                    let now = crate::helpers::now_iso();
                    let owner = NotificationRecipient {
                        id: crate::helpers::generate_id(),
                        address: tenant.email.to_lowercase(),
                        kind: RecipientKind::Cc,
                        status: RecipientStatus::Verified,
                        is_owner: true,
                        created_at: now.clone(),
                        verified_at: Some(now.clone()),
                    };
                    let new_addr = EmailAddress {
                        local_part: label.clone(),
                        tenant_id: tenant_id.to_string(),
                        auto_reply: ReplyConfig::default(),
                        notification_recipients: vec![owner],
                        created_at: now.clone(),
                        updated_at: now,
                    };
                    save_email_address(&kv, tenant_id, &new_addr).await?;
                    set_email_address_index(&kv, &label, tenant_id).await?;
                }
            }

            render_step("channels", &state, &kv, &env, tenant_id, base_url).await
        }

        // Remove an email address while in the wizard.
        "email/remove" => {
            let form: serde_json::Value = req.json().await?;
            let label = form
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();

            if !label.is_empty() {
                if delete_email_address(&kv, tenant_id, &label).await? {
                    delete_email_address_index(&kv, &label).await?;
                }
            }

            render_step("channels", &state, &kv, &env, tenant_id, base_url).await
        }

        // Save notification preferences
        "notifications" => {
            let form: serde_json::Value = req.json().await?;
            let is_true =
                |key: &str| -> bool { form.get(key).and_then(|v| v.as_str()) == Some("true") };

            let approval_discord = is_true("approval_discord");
            let approval_email = is_true("approval_email");
            if !approval_discord && !approval_email {
                return Response::from_html(
                    r#"<div class="error">Pick at least one approval channel: Discord or Email: so the AI knows where to ask before sending.</div>"#.to_string(),
                );
            }
            let cadence_raw = form
                .get("approval_cadence")
                .and_then(|v| v.as_str())
                .unwrap_or("hourly");
            state.notifications = NotificationConfig {
                approval_discord,
                approval_email,
                approval_email_cadence: crate::types::DigestCadence::from_str(cadence_raw),
            };
            state.step = "replies".to_string();
            save_onboarding(&kv, tenant_id, &state).await?;

            let headers = Headers::new();
            headers.set("HX-Redirect", &format!("{base_url}/admin/wizard/replies"))?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Save persona preset choice + default wait, advance to launch.
        // Detailed persona editing lives at /admin/persona.
        "replies/save" => {
            let form: serde_json::Value = req.json().await?;

            let preset_slug = form.get("preset_id").and_then(|v| v.as_str()).unwrap_or("");
            let preset =
                PersonaPreset::from_slug(preset_slug).unwrap_or(PersonaPreset::FriendlyFlorist);

            state.persona = PersonaConfig {
                source: PersonaSource::Preset(preset),
                safety: PersonaSafety::default(),
            };

            if let Some(n) = form.get("default_wait_seconds").and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            }) {
                state.default_wait_seconds = (n as u32).min(30);
            }

            // Apply the preset's bundled default rules to every channel
            // account this tenant has already connected. (New connections
            // pick up the same defaults via channel handler creation paths.)
            apply_preset_to_channels(&kv, tenant_id, preset, state.default_wait_seconds).await?;

            // Enqueue safety check for the preset's prompt.
            let job = crate::safety_queue::SafetyJob {
                tenant_id: tenant_id.to_string(),
                prompt_hash: state.persona.active_prompt_hash(),
            };
            state.persona.safety.status = PersonaSafetyStatus::Pending;
            state.step = "launch".to_string();
            save_onboarding(&kv, tenant_id, &state).await?;
            let _ = crate::safety_queue::enqueue(&env, job).await;

            let headers = Headers::new();
            headers.set("HX-Redirect", &format!("{base_url}/admin/wizard/launch"))?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Complete the wizard. Email subdomains left unpaid stay Suspended —
        // the dashboard surfaces them with a banner so the user can subscribe
        // later from Email Routing.
        "complete" => {
            state.completed = true;
            state.step = "launch".to_string();
            save_onboarding(&kv, tenant_id, &state).await?;

            let headers = Headers::new();
            headers.set("HX-Redirect", &format!("{base_url}/admin"))?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Default: show current step (resume from where user left off)
        _ => {
            let step =
                if state.step.is_empty() || state.step == "welcome" || state.step == "business" {
                    state.step = "basics".to_string();
                    let _ = save_onboarding(&kv, tenant_id, &state).await;
                    "basics"
                } else {
                    &state.step
                };
            let headers = Headers::new();
            headers.set("Location", &format!("{base_url}/admin/wizard/{step}"))?;
            Ok(Response::empty()?.with_status(302).with_headers(headers))
        }
    }
}

fn step_index(step: &str) -> usize {
    match step {
        "basics" => 0,
        "channels" => 1,
        "notifications" => 2,
        "replies" => 3,
        "launch" => 4,
        _ => 0,
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
            let email_addrs = get_email_addresses(kv, tenant_id).await?;
            let slug = crate::helpers::generate_slug().unwrap_or_else(|_| "my-biz".into());
            let base_domain = env
                .var("EMAIL_BASE_DOMAIN")
                .map(|v| v.to_string())
                .unwrap_or_default();
            let db = env.d1("DB")?;
            let currency = crate::storage::get_tenant(&db, tenant_id)
                .await?
                .map(|t| t.currency)
                .unwrap_or_else(|| "INR".to_string());
            let discord = get_discord_config_by_tenant(kv, tenant_id).await?;
            Response::from_html(connect_html(
                !ig.is_empty(),
                !wa.is_empty(),
                &email_addrs,
                &slug,
                &base_domain,
                &currency,
                tenant_id,
                discord.as_ref(),
                base_url,
            ))
        }
        "notifications" => {
            let dc_installed = get_discord_config_by_tenant(kv, tenant_id).await?.is_some();
            Response::from_html(notifications_html(
                &state.notifications,
                dc_installed,
                base_url,
            ))
        }
        "replies" => Response::from_html(replies_html(
            &state.persona,
            state.default_wait_seconds,
            base_url,
        )),
        "launch" => {
            let email_addrs = get_email_addresses(kv, tenant_id).await?;
            let base_domain = env
                .var("EMAIL_BASE_DOMAIN")
                .map(|v| v.to_string())
                .unwrap_or_default();
            let db = env.d1("DB")?;
            let tenant = crate::storage::get_tenant(&db, tenant_id)
                .await?
                .unwrap_or_default();
            let locale = crate::locale::Locale::from_tenant(&tenant.locale, Some(&tenant.currency));
            Response::from_html(launch_html(&email_addrs, &base_domain, &locale, base_url))
        }
        _ => Response::from_html(basics_html(&state.business, base_url)),
    }
}

/// Seed every already-connected channel's `ReplyConfig` with the preset's
/// bundled rules + wait_seconds. Existing per-rule overrides are replaced
/// since the user has just chosen a fresh starting style.
async fn apply_preset_to_channels(
    kv: &kv::KvStore,
    tenant_id: &str,
    preset: PersonaPreset,
    wait_seconds: u32,
) -> Result<()> {
    let now = crate::helpers::now_iso();
    let rules = preset.default_rules();

    let wa = list_whatsapp_accounts(kv, tenant_id).await?;
    for mut acct in wa {
        acct.auto_reply.rules = rules.clone();
        acct.auto_reply.wait_seconds = wait_seconds;
        acct.updated_at = now.clone();
        save_whatsapp_account(kv, &acct).await?;
    }

    let ig = list_instagram_accounts(kv, tenant_id).await?;
    for mut acct in ig {
        acct.auto_reply.rules = rules.clone();
        acct.auto_reply.wait_seconds = wait_seconds;
        acct.updated_at = now.clone();
        save_instagram_account(kv, &acct).await?;
    }

    let emails = get_email_addresses(kv, tenant_id).await?;
    for mut addr in emails {
        addr.auto_reply.rules = rules.clone();
        addr.auto_reply.wait_seconds = wait_seconds;
        addr.updated_at = now.clone();
        save_email_address(kv, tenant_id, &addr).await?;
    }

    if let Some(mut dc) = get_discord_config_by_tenant(kv, tenant_id).await? {
        dc.auto_reply.rules = rules.clone();
        dc.auto_reply.wait_seconds = wait_seconds;
        save_discord_config(kv, &dc).await?;
    }

    Ok(())
}
