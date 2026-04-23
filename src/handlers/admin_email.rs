//! Admin handlers for email routing: domain + rule CRUD

use worker::*;

use crate::helpers::*;
use crate::storage::*;
use crate::templates::admin_email::*;
use crate::types::*;

/// Handle /admin/email routes
pub async fn handle_email_admin(
    mut req: Request,
    env: Env,
    path: &str,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    let path_parts: Vec<&str> = path
        .strip_prefix("/admin/email")
        .unwrap_or("")
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let method = req.method();

    match (method, path_parts.as_slice()) {
        // Dashboard: list subdomains + metrics
        (Method::Get, []) => {
            let subdomains = get_email_subdomains(&kv, tenant_id).await?;
            let metrics = get_email_metrics(&db, tenant_id, None).await?;
            let email_base_domain = env
                .var("EMAIL_BASE_DOMAIN")
                .map(|v| v.to_string())
                .unwrap_or_default();
            let currency = get_tenant(&db, tenant_id)
                .await?
                .map(|t| t.currency)
                .unwrap_or_else(|| "INR".to_string());
            Response::from_html(email_dashboard_html(
                &subdomains,
                &metrics,
                &email_base_domain,
                &currency,
                base_url,
            ))
        }

        // Log viewer
        (Method::Get, ["log"]) => {
            let log = get_email_log(&db, tenant_id, 100).await?;
            Response::from_html(email_log_html(&log, base_url))
        }

        // Settings (discord bot token)
        (Method::Get, ["settings"]) => {
            let token = get_discord_bot_token(&kv, tenant_id).await?;
            Response::from_html(email_settings_html(token.as_deref(), base_url))
        }

        // Save discord bot token
        (Method::Put, ["settings"]) => {
            let form: serde_json::Value = req.json().await?;
            if let Some(token) = form.get("discord_bot_token").and_then(|v| v.as_str()) {
                save_discord_bot_token(&kv, tenant_id, token).await?;
            }
            Response::from_html("<div class=\"success\">Settings saved</div>".to_string())
        }

        // Add or subscribe to a subdomain. The wizard's "channels" step can
        // pre-create a subdomain with no subscription yet — so if a row for
        // this label already exists without a subscription, resume it here
        // instead of erroring.
        (Method::Post, ["subdomains"]) => {
            let form: serde_json::Value = req.json().await?;
            let label = form
                .get("subdomain")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();

            let base_domain = env
                .var("EMAIL_BASE_DOMAIN")
                .map(|v| v.to_string())
                .unwrap_or_default();

            if let Err(e) = crate::cloudflare::dns::validate_subdomain_label(&label) {
                return Response::from_html(format!(
                    "<div class=\"error\">{}</div>",
                    crate::helpers::html_escape(e)
                ));
            }

            let domain = format!("{label}.{base_domain}");

            let mut subdomains = get_email_subdomains(&kv, tenant_id).await?;
            let existing_idx = subdomains.iter().position(|d| d.domain == domain);
            if let Some(idx) = existing_idx {
                if subdomains[idx].subscription_id.is_some() {
                    return Response::from_html(
                        "<div class=\"error\">Subdomain already subscribed</div>".to_string(),
                    );
                }
            } else if get_tenant_by_domain(&kv, &domain).await?.is_some() {
                // Only check global uniqueness when creating fresh
                return Response::from_html(
                    "<div class=\"error\">Subdomain is already taken</div>".to_string(),
                );
            }

            // Get or create a Razorpay plan for this tenant's total subdomain count
            let key_id = env.secret("RAZORPAY_KEY_ID")?.to_string();
            let key_secret = env.secret("RAZORPAY_KEY_SECRET")?.to_string();

            // Price per subdomain: ₹199 (19900 paise) or $2 (200 cents)
            let tenant =
                get_tenant(&db, tenant_id)
                    .await?
                    .unwrap_or_else(|| crate::types::Tenant {
                        id: tenant_id.to_string(),
                        currency: "INR".to_string(),
                        ..Default::default()
                    });
            let new_count = (subdomains.len() + 1) as i64;
            let (per_unit, currency): (i64, &str) = if tenant.currency == "USD" {
                (200, "USD")
            } else {
                (19900, "INR")
            };
            let amount = per_unit * new_count;

            let plan_key = format!("razorpay_plan:email:{currency}:{amount}");
            let plan_id = match kv.get(&plan_key).text().await? {
                Some(id) => id,
                None => {
                    let name = format!("Email {} subdomain(s)", new_count);
                    let plan = crate::billing::razorpay::create_plan(
                        &key_id,
                        &key_secret,
                        amount,
                        currency,
                        "monthly",
                        1,
                        &name,
                    )
                    .await?;
                    let id = plan
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if id.is_empty() {
                        return Response::from_html(
                            "<div class=\"error\">Failed to create billing plan</div>".to_string(),
                        );
                    }
                    kv.put(&plan_key, &id)?.execute().await?;
                    console_log!("Created Razorpay plan {id} for {currency} {amount}");
                    id
                }
            };

            let subscription = crate::billing::razorpay::create_subscription(
                &key_id,
                &key_secret,
                &plan_id,
                tenant_id,
                &label,
            )
            .await?;

            let subscription_id = subscription
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let short_url = subscription
                .get("short_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if subscription_id.is_empty() || short_url.is_empty() {
                return Response::from_html(
                    "<div class=\"error\">Failed to create subscription. Please try again.</div>"
                        .to_string(),
                );
            }

            // Record the subscription against an existing pending row, or
            // create a fresh Suspended row. MX records are provisioned by the
            // Razorpay webhook on payment confirmation.
            let now = now_iso();
            match existing_idx {
                Some(idx) => {
                    subdomains[idx].subscription_id = Some(subscription_id);
                    subdomains[idx].updated_at = now.clone();
                }
                None => {
                    subdomains.push(EmailSubdomain {
                        label: label.clone(),
                        domain: domain.clone(),
                        tenant_id: tenant_id.to_string(),
                        default_action: EmailAction::Drop,
                        dns_record_ids: vec![],
                        subscription_id: Some(subscription_id),
                        status: SubdomainStatus::Suspended,
                        created_at: now.clone(),
                        updated_at: now,
                    });
                }
            }
            save_email_subdomains(&kv, tenant_id, &subdomains).await?;

            // Redirect to Razorpay payment page
            let headers = Headers::new();
            headers.set("HX-Redirect", &short_url)?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Delete subdomain
        (Method::Delete, ["subdomains", label]) => {
            let mut subdomains = get_email_subdomains(&kv, tenant_id).await?;
            let removed = subdomains.iter().find(|d| d.label == *label).cloned();
            subdomains.retain(|d| d.label != *label);
            save_email_subdomains(&kv, tenant_id, &subdomains).await?;

            if let Some(sub) = removed {
                delete_email_domain_index(&kv, &sub.domain).await?;
                save_email_rules(&kv, tenant_id, &sub.domain, &[]).await?;

                // Cancel Razorpay subscription
                if let Some(ref sub_id) = sub.subscription_id {
                    let key_id = env.secret("RAZORPAY_KEY_ID")?.to_string();
                    let key_secret = env.secret("RAZORPAY_KEY_SECRET")?.to_string();
                    let _ =
                        crate::billing::razorpay::cancel_subscription(&key_id, &key_secret, sub_id)
                            .await;
                }

                // Delete MX records from Cloudflare
                if !sub.dns_record_ids.is_empty() {
                    let zone_id = env
                        .var("EMAIL_ZONE_ID")
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    let api_token = env.secret("CF_DNS_API_TOKEN")?.to_string();
                    let _ = crate::cloudflare::dns::delete_dns_records(
                        &zone_id,
                        &sub.dns_record_ids,
                        &api_token,
                    )
                    .await;
                }
            }

            Ok(Response::empty()?.with_status(200))
        }

        // List rules for a domain
        (Method::Get, ["domains", domain, "rules"]) => {
            let rules = get_email_rules(&kv, tenant_id, domain).await?;
            Response::from_html(email_rules_html(domain, &rules, base_url))
        }

        // Add rule
        (Method::Post, ["domains", domain, "rules"]) => {
            let form: serde_json::Value = req.json().await?;
            let mut rules = get_email_rules(&kv, tenant_id, domain).await?;

            let rule = parse_rule_from_json(domain, &form)?;
            rules.push(rule);
            rules.sort_by_key(|r| r.priority);
            save_email_rules(&kv, tenant_id, domain, &rules).await?;

            let headers = Headers::new();
            headers.set(
                "HX-Redirect",
                &format!("{base_url}/admin/email/domains/{domain}/rules"),
            )?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Edit rule page
        (Method::Get, ["domains", domain, "rules", rule_id]) => {
            let rules = get_email_rules(&kv, tenant_id, domain).await?;
            match rules.iter().find(|r| r.id == *rule_id) {
                Some(rule) => Response::from_html(email_rule_edit_html(domain, rule, base_url)),
                None => Response::error("Rule not found", 404),
            }
        }

        // Update rule
        (Method::Put, ["domains", domain, "rules", rule_id]) => {
            let form: serde_json::Value = req.json().await?;
            let mut rules = get_email_rules(&kv, tenant_id, domain).await?;

            if let Some(existing) = rules.iter_mut().find(|r| r.id == *rule_id) {
                let updated = parse_rule_from_json(domain, &form)?;
                existing.name = updated.name;
                existing.priority = updated.priority;
                existing.enabled = updated.enabled;
                existing.criteria = updated.criteria;
                existing.action = updated.action;
                existing.updated_at = now_iso();
            }

            rules.sort_by_key(|r| r.priority);
            save_email_rules(&kv, tenant_id, domain, &rules).await?;

            Response::from_html("<div class=\"success\">Rule updated</div>".to_string())
        }

        // Delete rule
        (Method::Delete, ["domains", domain, "rules", rule_id]) => {
            let mut rules = get_email_rules(&kv, tenant_id, domain).await?;
            rules.retain(|r| r.id != *rule_id);
            save_email_rules(&kv, tenant_id, domain, &rules).await?;

            Ok(Response::empty()?.with_status(200))
        }

        // Toggle rule enabled/disabled
        (Method::Post, ["domains", domain, "rules", rule_id, "toggle"]) => {
            let mut rules = get_email_rules(&kv, tenant_id, domain).await?;
            if let Some(rule) = rules.iter_mut().find(|r| r.id == *rule_id) {
                rule.enabled = !rule.enabled;
                rule.updated_at = now_iso();
            }
            save_email_rules(&kv, tenant_id, domain, &rules).await?;

            let headers = Headers::new();
            headers.set(
                "HX-Redirect",
                &format!("{base_url}/admin/email/domains/{domain}/rules"),
            )?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        _ => Response::error("Not Found", 404),
    }
}

/// Parse a routing rule from JSON form data.
fn parse_rule_from_json(domain: &str, form: &serde_json::Value) -> Result<RoutingRule> {
    let name = form
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unnamed rule")
        .to_string();
    let priority = form
        .get("priority")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let enabled = form.get("enabled").and_then(|v| v.as_str()) == Some("true");

    let criteria = MatchCriteria {
        from_pattern: non_empty_str(form, "from_pattern"),
        to_pattern: non_empty_str(form, "to_pattern"),
        subject_pattern: non_empty_str(form, "subject_pattern"),
        has_attachment: form
            .get("has_attachment")
            .and_then(|v| v.as_str())
            .map(|v| v == "true"),
        body_pattern: non_empty_str(form, "body_pattern"),
    };

    let action_type = form
        .get("action_type")
        .and_then(|v| v.as_str())
        .unwrap_or("drop");

    let action = match action_type {
        "drop" => EmailAction::Drop,
        "spam" => EmailAction::Spam {
            message: non_empty_str(form, "spam_message"),
        },
        "forward_email" => EmailAction::ForwardEmail {
            destination: form
                .get("destination")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        },
        "forward_discord" => EmailAction::ForwardDiscord {
            channel_id: form
                .get("channel_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        },
        "ai_reply" => EmailAction::AiReply {
            system_prompt: non_empty_str(form, "system_prompt"),
            approval_channel_id: non_empty_str(form, "approval_channel_id"),
            approval_email: non_empty_str(form, "approval_email"),
        },
        _ => EmailAction::Drop,
    };

    let now = now_iso();
    Ok(RoutingRule {
        id: generate_id(),
        domain: domain.to_string(),
        name,
        priority,
        enabled,
        criteria,
        action,
        created_at: now.clone(),
        updated_at: now,
    })
}

fn non_empty_str(form: &serde_json::Value, key: &str) -> Option<String> {
    form.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
}
