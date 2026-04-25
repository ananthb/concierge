//! Admin handlers for the email feature: addresses, auto-reply config,
//! notification recipients (with verification email).
//!
//! Per-customer subdomains and the legacy routing-rule engine are gone: see
//! `doc/architecture.html` for the new model.

use worker::*;

use crate::email::{send::OutboundEmail, validate_local_part};
use crate::helpers::*;
use crate::storage::*;
use crate::templates::admin_email::*;
use crate::types::*;

/// Handle `/admin/email/*` routes.
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
    let base_domain = env
        .var("EMAIL_BASE_DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_default();

    match (method, path_parts.as_slice()) {
        // Dashboard.
        (Method::Get, []) => {
            let addrs = get_email_addresses(&kv, tenant_id).await?;
            let tenant = get_tenant(&db, tenant_id).await?.unwrap_or_default();
            Response::from_html(email_dashboard_html(
                &addrs,
                &tenant,
                &base_domain,
                base_url,
            ))
        }

        // Add an address. Body: { local_part: "support" }
        (Method::Post, ["addresses"]) => {
            let form: serde_json::Value = req.json().await?;
            let label = form
                .get("local_part")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();

            if let Err(e) = validate_local_part(&label) {
                return Response::from_html(format!(
                    r#"<div class="error">{}</div>"#,
                    html_escape(e)
                ));
            }

            // Quota check.
            let tenant = get_tenant(&db, tenant_id).await?.unwrap_or_default();
            let addrs = get_email_addresses(&kv, tenant_id).await?;
            if (addrs.len() as u32) >= tenant.email_address_quota() {
                return Response::from_html(
                    r#"<div class="error">Address quota reached. <a href="/admin/billing">Buy more</a> to add additional addresses.</div>"#
                        .to_string(),
                );
            }

            // Global uniqueness: local-parts are shared across the platform.
            if get_tenant_by_address(&kv, &label).await?.is_some() {
                return Response::from_html(
                    r#"<div class="error">That address is already taken.</div>"#.to_string(),
                );
            }

            // Create with the owner email pre-listed as a verified Cc
            // recipient. We don't need a verification round-trip for the
            // tenant's own login email.
            let now = now_iso();
            let owner = NotificationRecipient {
                id: generate_id(),
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
                auto_reply: AutoReplyConfig::default(),
                notification_recipients: vec![owner],
                created_at: now.clone(),
                updated_at: now,
            };
            save_email_address(&kv, tenant_id, &new_addr).await?;
            set_email_address_index(&kv, &label, tenant_id).await?;

            let headers = Headers::new();
            headers.set("HX-Redirect", &format!("{base_url}/admin/email"))?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Delete an address.
        (Method::Delete, ["addresses", label]) => {
            let removed = delete_email_address(&kv, tenant_id, label).await?;
            if removed {
                delete_email_address_index(&kv, label).await?;
            }
            Ok(Response::empty()?.with_status(200))
        }

        // Per-address edit page.
        (Method::Get, ["addresses", label]) => {
            let addr = match get_email_address(&kv, tenant_id, label).await? {
                Some(a) => a,
                None => return Response::error("Not found", 404),
            };
            Response::from_html(email_address_html(&addr, &base_domain, base_url))
        }

        // Update auto-reply config.
        (Method::Put, ["addresses", label, "auto-reply"]) => {
            let form: serde_json::Value = req.json().await?;
            let mut addr = match get_email_address(&kv, tenant_id, label).await? {
                Some(a) => a,
                None => return Response::error("Not found", 404),
            };

            addr.auto_reply.enabled = form
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let mode_str = form
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("static");
            addr.auto_reply.mode = if mode_str == "ai" {
                AutoReplyMode::Ai
            } else {
                AutoReplyMode::Static
            };
            addr.auto_reply.prompt = form
                .get("prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            addr.auto_reply.wait_seconds = form
                .get("wait_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(5) as u32;
            addr.updated_at = now_iso();

            save_email_address(&kv, tenant_id, &addr).await?;

            let headers = Headers::new();
            headers.set(
                "HX-Redirect",
                &format!("{base_url}/admin/email/addresses/{label}"),
            )?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Add a notification recipient (Cc or Bcc). Sends a verification
        // email unless the address matches the tenant's owner login.
        (Method::Post, ["addresses", label, "recipients"]) => {
            let form: serde_json::Value = req.json().await?;
            let address = form
                .get("address")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();
            let kind_str = form.get("kind").and_then(|v| v.as_str()).unwrap_or("cc");

            if !address.contains('@') || address.len() < 3 {
                return Response::from_html(
                    r#"<div class="error">Enter a valid email address.</div>"#.to_string(),
                );
            }
            let kind = if kind_str == "bcc" {
                RecipientKind::Bcc
            } else {
                RecipientKind::Cc
            };

            let mut addr = match get_email_address(&kv, tenant_id, label).await? {
                Some(a) => a,
                None => return Response::error("Not found", 404),
            };

            if addr
                .notification_recipients
                .iter()
                .any(|r| r.address == address && r.kind == kind)
            {
                return Response::from_html(
                    r#"<div class="error">Already in the list.</div>"#.to_string(),
                );
            }

            let tenant = get_tenant(&db, tenant_id).await?.unwrap_or_default();
            let owner_email = tenant.email.to_lowercase();
            let is_owner = address == owner_email;
            let now = now_iso();
            let recipient_id = generate_id();
            let status = if is_owner {
                RecipientStatus::Verified
            } else {
                RecipientStatus::Pending
            };
            let verified_at = if is_owner { Some(now.clone()) } else { None };

            let recipient = NotificationRecipient {
                id: recipient_id.clone(),
                address: address.clone(),
                kind,
                status: status.clone(),
                is_owner,
                created_at: now.clone(),
                verified_at,
            };
            addr.notification_recipients.push(recipient);
            addr.updated_at = now;
            save_email_address(&kv, tenant_id, &addr).await?;

            // Send verification email for non-owner additions.
            if !is_owner {
                let token = generate_id();
                let payload = EmailVerificationPayload {
                    tenant_id: tenant_id.to_string(),
                    local_part: label.to_string(),
                    recipient_id,
                };
                set_email_verification_token(&kv, &token, &payload).await?;

                let from = format!("noreply@{base_domain}");
                let concierge_addr = format!("{label}@{base_domain}");
                let verify_url = format!("{base_url}/email/verify/{token}");
                let body = format!(
                    "Concierge sends customer replies from {concierge_addr}. To start receiving notifications, confirm this address by opening:\n\n{verify_url}\n\nIf you weren't expecting this, you can ignore the message: the request will expire in 7 days.\n",
                );
                let outbound = OutboundEmail {
                    from,
                    to: address,
                    subject: format!("Confirm notifications from {concierge_addr}"),
                    text: Some(body),
                    html: None,
                    reply_to: None,
                    cc: vec![],
                    bcc: vec![],
                    headers: vec![("X-EmailProxy-Forwarded".to_string(), "1".to_string())],
                };
                if let Err(e) = crate::email::send::send_outbound(&env, &outbound).await {
                    console_log!("Failed to send verification email: {:?}", e);
                }
            }

            let headers = Headers::new();
            headers.set(
                "HX-Redirect",
                &format!("{base_url}/admin/email/addresses/{label}"),
            )?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Delete a notification recipient. Owner cannot be deleted.
        (Method::Delete, ["addresses", label, "recipients", id]) => {
            let mut addr = match get_email_address(&kv, tenant_id, label).await? {
                Some(a) => a,
                None => return Response::error("Not found", 404),
            };
            let id = id.to_string();
            let was_owner = addr
                .notification_recipients
                .iter()
                .find(|r| r.id == id)
                .map(|r| r.is_owner)
                .unwrap_or(false);
            if was_owner {
                return Response::from_html(
                    r#"<div class="error">The owner email can't be removed.</div>"#.to_string(),
                );
            }
            addr.notification_recipients.retain(|r| r.id != id);
            addr.updated_at = now_iso();
            save_email_address(&kv, tenant_id, &addr).await?;
            Ok(Response::empty()?.with_status(200))
        }

        _ => Response::error("Not found", 404),
    }
}
