//! Main email event handler — receives inbound emails, matches rules, executes actions.

use worker::*;

use super::{ai_reply, discord, forward, mime, routing};
use crate::helpers::generate_id;
use crate::storage::*;
use crate::types::*;

/// Handle an incoming email. Called from the wasm_bindgen email() export.
pub async fn handle_email(
    from: &str,
    to: &str,
    raw_bytes: &[u8],
    env: &Env,
) -> Result<EmailResult> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    // Extract domain from recipient
    let domain = to.rsplit('@').next().unwrap_or("").to_lowercase();

    if domain.is_empty() {
        return Ok(EmailResult::Reject("Invalid recipient".into()));
    }

    // Loop detection: check for our forwarding header in the raw email headers.
    // Only search up to the first blank line (header/body boundary) to avoid false positives.
    let header_end = raw_bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(raw_bytes.len().min(8192));
    let header_section = &raw_bytes[..header_end];
    let header_lower: Vec<u8> = header_section
        .iter()
        .map(|b| b.to_ascii_lowercase())
        .collect();
    if header_lower
        .windows(b"x-emailproxy-forwarded:".len())
        .any(|w| w == b"x-emailproxy-forwarded:")
    {
        console_log!("Loop detected: {from} -> {to}");
        return Ok(EmailResult::Reject("Forwarding loop detected".into()));
    }

    // Look up tenant by domain
    let tenant_id = match get_tenant_by_domain(&kv, &domain).await? {
        Some(id) => id,
        None => {
            console_log!("No tenant for domain: {domain}");
            return Ok(EmailResult::Reject("Unknown domain".into()));
        }
    };

    // Parse the email
    let parsed = mime::parse_email(raw_bytes);
    let subject = parsed.as_ref().map(|p| p.subject.as_str()).unwrap_or("");
    let text_body = parsed
        .as_ref()
        .and_then(|p| p.text_body.as_deref())
        .unwrap_or("");
    let has_attachment = parsed.as_ref().map(|p| p.has_attachment).unwrap_or(false);

    // Check if this is a reverse alias reply
    if forward::is_reverse_alias(to) {
        return handle_reverse_alias(from, to, &parsed, &kv, &db, &domain, &tenant_id).await;
    }

    // Load and match routing rules
    let rules = get_email_rules(&kv, &tenant_id, &domain).await?;
    let matched_rule =
        routing::find_matching_rule(&rules, from, to, subject, has_attachment, text_body);

    let (rule_id, rule_name, action) = match matched_rule {
        Some(rule) => (Some(rule.id.as_str()), rule.name.as_str(), &rule.action),
        None => {
            // Use domain default action
            let domains = get_email_subdomains(&kv, &tenant_id).await?;
            let domain_config = domains.iter().find(|d| d.domain == domain);
            let default = domain_config
                .map(|d| d.default_action.clone())
                .unwrap_or(EmailAction::Drop);
            // Store temporarily to extend lifetime
            return execute_action(
                &default, None, "default", from, to, subject, text_body, &parsed, &kv, &db,
                &domain, &tenant_id, env,
            )
            .await;
        }
    };

    execute_action(
        action, rule_id, rule_name, from, to, subject, text_body, &parsed, &kv, &db, &domain,
        &tenant_id, env,
    )
    .await
}

/// Execute a matched action.
async fn execute_action(
    action: &EmailAction,
    rule_id: Option<&str>,
    rule_name: &str,
    from: &str,
    to: &str,
    subject: &str,
    text_body: &str,
    parsed: &Option<mime::ParsedEmail>,
    kv: &kv::KvStore,
    db: &D1Database,
    domain: &str,
    tenant_id: &str,
    env: &Env,
) -> Result<EmailResult> {
    let log_id = generate_id();
    let action_name = match action {
        EmailAction::Drop => "dropped",
        EmailAction::Spam { .. } => "spam",
        EmailAction::ForwardEmail { .. } => "forwarded",
        EmailAction::ForwardDiscord { .. } => "discord",
        EmailAction::AiReply { .. } => "ai_reply",
    };

    let result = match action {
        EmailAction::Drop => {
            console_log!("Dropping email from {from} to {to} (rule: {rule_name})");
            Ok(EmailResult::Drop)
        }

        EmailAction::Spam { message } => {
            let msg = message.as_deref().unwrap_or("Rejected");
            console_log!("Rejecting as spam from {from} to {to} (rule: {rule_name})");
            Ok(EmailResult::Reject(msg.to_string()))
        }

        EmailAction::ForwardEmail { destination } => {
            if let Some(ref parsed) = parsed {
                let reverse_addr = forward::generate_reverse_address(domain);

                // Save reverse alias for reply routing
                let reverse_alias = EmailReverseAlias {
                    alias: to.to_string(),
                    original_sender: from.to_string(),
                    tenant_id: tenant_id.to_string(),
                    domain: domain.to_string(),
                };
                save_email_reverse_alias(kv, &reverse_addr, &reverse_alias).await?;

                let forwarded = mime::build_forwarded_email(
                    parsed,
                    &reverse_addr,
                    destination,
                    &reverse_addr,
                    from,
                    domain,
                );

                console_log!(
                    "Forwarding from {from} to {destination} via {to} (rule: {rule_name})"
                );
                Ok(EmailResult::Send {
                    from: reverse_addr,
                    to: destination.clone(),
                    raw: forwarded,
                })
            } else {
                Ok(EmailResult::Reject("Failed to parse email".into()))
            }
        }

        EmailAction::ForwardDiscord { channel_id } => {
            if let Some(token) = get_discord_bot_token(kv, tenant_id).await? {
                discord::post_email_to_discord(
                    &token, channel_id, from, to, subject, text_body, rule_name,
                )
                .await?;
                console_log!("Forwarded to Discord channel {channel_id} (rule: {rule_name})");
            } else {
                console_log!("Discord bot token not configured for tenant {tenant_id}");
            }
            Ok(EmailResult::Drop) // Email is consumed by Discord, no SMTP forward
        }

        EmailAction::AiReply {
            system_prompt,
            approval_channel_id,
            approval_email: _,
        } => {
            let draft = ai_reply::generate_email_reply(
                env,
                system_prompt.as_deref(),
                from,
                subject,
                text_body,
            )
            .await?;

            // Post draft to Discord for approval
            if let Some(ch_id) = approval_channel_id {
                if let Some(token) = get_discord_bot_token(kv, tenant_id).await? {
                    discord::post_ai_draft_for_approval(
                        &token, ch_id, from, to, subject, text_body, &draft, rule_name,
                    )
                    .await?;
                    console_log!("AI draft posted to Discord for approval (rule: {rule_name})");
                }
            }

            // For now, AI replies are always held for approval — not auto-sent
            Ok(EmailResult::Drop)
        }
    };

    // Log the email
    let _ = save_email_message(
        db,
        &EmailLogEntry {
            id: &log_id,
            tenant_id,
            domain,
            rule_id,
            direction: "inbound",
            from_email: from,
            to_email: to,
            action_taken: action_name,
            error_msg: None,
        },
    )
    .await;

    // Increment metrics
    let _ = increment_email_metric(db, domain, rule_id, action_name, tenant_id).await;

    result
}

/// Handle a reply arriving at a reverse alias.
async fn handle_reverse_alias(
    from: &str,
    to: &str,
    parsed: &Option<mime::ParsedEmail>,
    kv: &kv::KvStore,
    db: &D1Database,
    domain: &str,
    tenant_id: &str,
) -> Result<EmailResult> {
    let log_id = generate_id();

    let reverse = match get_email_reverse_alias(kv, to).await? {
        Some(r) => r,
        None => {
            console_log!("Unknown reverse alias: {to}");
            return Ok(EmailResult::Reject("Unknown reverse alias".into()));
        }
    };

    let parsed = match parsed {
        Some(p) => p,
        None => {
            return Ok(EmailResult::Reject("Failed to parse reply".into()));
        }
    };

    let reply_bytes =
        mime::build_reply_email(parsed, &reverse.alias, &reverse.original_sender, domain);

    let _ = save_email_message(
        db,
        &EmailLogEntry {
            id: &log_id,
            tenant_id,
            domain,
            rule_id: None,
            direction: "reply",
            from_email: &reverse.alias,
            to_email: &reverse.original_sender,
            action_taken: "forwarded",
            error_msg: None,
        },
    )
    .await;

    console_log!(
        "Reply routing: {from} -> {} -> {}",
        reverse.alias,
        reverse.original_sender
    );

    Ok(EmailResult::Send {
        from: reverse.alias,
        to: reverse.original_sender,
        raw: reply_bytes,
    })
}

/// Result of email processing.
pub enum EmailResult {
    /// Silently drop the email.
    Drop,
    /// Reject the email with a message.
    Reject(String),
    /// Send a new email via the send_email binding.
    Send {
        from: String,
        to: String,
        raw: Vec<u8>,
    },
}
