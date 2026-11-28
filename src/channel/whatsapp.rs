use worker::*;

use crate::helpers::generate_id;
use crate::types::*;

/// Parse WhatsApp webhook change into InboundMessages.
pub fn parse_inbound(change: &WebhookChange, account: &WhatsAppAccount) -> Vec<InboundMessage> {
    let phone_number_id = &change.value.metadata.phone_number_id;
    let contacts = &change.value.contacts;

    change
        .value
        .messages
        .iter()
        .filter_map(|msg| {
            let text = msg.text.as_ref()?.body.clone();
            let sender_name = contacts
                .iter()
                .find(|c| c.wa_id == msg.from)
                .map(|c| c.profile.name.clone());

            Some(InboundMessage {
                id: generate_id(),
                channel: Channel::WhatsApp,
                sender: msg.from.clone(),
                sender_name,
                recipient: phone_number_id.clone(),
                reply_to: msg.from.clone(),
                body: text,
                has_attachment: false,
                tenant_id: account.tenant_id.clone(),
                channel_account_id: account.id.clone(),
                raw_metadata: serde_json::json!({
                    "phone_number_id": phone_number_id,
                    "whatsapp_account_id": account.id,
                    "message_id": msg.id,
                }),
            })
        })
        .collect()
}

fn resolve_tokens(env: &Env, account_token: Option<&str>) -> Vec<String> {
    let mut tokens = Vec::new();
    if let Some(tok) = account_token.map(str::trim).filter(|s| !s.is_empty()) {
        tokens.push(tok.to_string());
    }
    if let Ok(secret) = env.secret("WHATSAPP_ACCESS_TOKEN") {
        let platform = secret.to_string().trim().to_string();
        if !platform.is_empty() && !tokens.iter().any(|t| t == &platform) {
            tokens.push(platform);
        }
    }
    tokens
}

/// Send a reply via WhatsApp.
pub async fn send_reply(
    env: &Env,
    metadata: &serde_json::Value,
    to: &str,
    body: &str,
    account_token: Option<&str>,
) -> Result<()> {
    let phone_number_id = metadata
        .get("phone_number_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let tokens = resolve_tokens(env, account_token);
    let Some((first, rest)) = tokens.split_first() else {
        return crate::whatsapp::send_whatsapp_message("", phone_number_id, to, body).await;
    };

    let mut last_err =
        match crate::whatsapp::send_whatsapp_message(first, phone_number_id, to, body).await {
            Ok(()) => return Ok(()),
            Err(e) => e,
        };
    for tok in rest {
        match crate::whatsapp::send_whatsapp_message(tok, phone_number_id, to, body).await {
            Ok(()) => return Ok(()),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

/// Send a WhatsApp read receipt + typing indicator ("...") for an inbound message.
/// Returns `Ok(true)` if an indicator request succeeded, `Ok(false)` otherwise.
/// Failures are swallowed so a typing indicator error never aborts a reply.
pub async fn send_typing_indicator(
    env: &Env,
    metadata: &serde_json::Value,
    account_token: Option<&str>,
) -> Result<bool> {
    let phone_number_id = metadata
        .get("phone_number_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("");
    let message_id = metadata
        .get("message_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("");
    if phone_number_id.is_empty() || message_id.is_empty() {
        return Ok(false);
    }

    for tok in resolve_tokens(env, account_token) {
        if crate::whatsapp::send_whatsapp_typing_indicator(&tok, phone_number_id, message_id)
            .await
            .is_ok()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_account() -> WhatsAppAccount {
        WhatsAppAccount {
            id: "wa-acc-1".into(),
            tenant_id: "tenant-1".into(),
            name: "Test".into(),
            phone_number: "+1234567890".into(),
            phone_number_id: "phone-123".into(),
            waba_id: Some("waba-123".into()),
            auto_reply: ReplyConfig::default(),
            canned_safety: SafetyState::default(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn make_change(messages: Vec<WhatsAppMessage>, contacts: Vec<WebhookContact>) -> WebhookChange {
        WebhookChange {
            field: "messages".into(),
            value: WebhookValue {
                messaging_product: "whatsapp".into(),
                metadata: WebhookMetadata {
                    display_phone_number: "+1234567890".into(),
                    phone_number_id: "phone-123".into(),
                },
                contacts,
                messages,
            },
        }
    }

    #[test]
    fn parse_text_message() {
        let change = make_change(
            vec![WhatsAppMessage {
                from: "user-1".into(),
                id: "msg-1".into(),
                timestamp: "1234567890".into(),
                message_type: "text".into(),
                text: Some(TextMessage {
                    body: "Hello!".into(),
                }),
            }],
            vec![WebhookContact {
                wa_id: "user-1".into(),
                profile: ContactProfile {
                    name: "Alice".into(),
                },
            }],
        );

        let msgs = parse_inbound(&change, &make_account());
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].channel, Channel::WhatsApp);
        assert_eq!(msgs[0].sender, "user-1");
        assert_eq!(msgs[0].reply_to, "user-1");
        assert_eq!(msgs[0].conversation_key(), "user-1");
        assert_eq!(msgs[0].sender_name.as_deref(), Some("Alice"));
        assert_eq!(msgs[0].body, "Hello!");
        assert_eq!(msgs[0].recipient, "phone-123");
        assert_eq!(msgs[0].tenant_id, "tenant-1");
    }

    #[test]
    fn skip_non_text_messages() {
        let change = make_change(
            vec![WhatsAppMessage {
                from: "user-1".into(),
                id: "msg-1".into(),
                timestamp: "1234567890".into(),
                message_type: "image".into(),
                text: None,
            }],
            vec![],
        );

        let msgs = parse_inbound(&change, &make_account());
        assert!(msgs.is_empty());
    }

    #[test]
    fn multiple_messages_in_one_change() {
        let change = make_change(
            vec![
                WhatsAppMessage {
                    from: "user-1".into(),
                    id: "msg-1".into(),
                    timestamp: "1".into(),
                    message_type: "text".into(),
                    text: Some(TextMessage { body: "Hi".into() }),
                },
                WhatsAppMessage {
                    from: "user-2".into(),
                    id: "msg-2".into(),
                    timestamp: "2".into(),
                    message_type: "text".into(),
                    text: Some(TextMessage { body: "Hey".into() }),
                },
            ],
            vec![],
        );

        let msgs = parse_inbound(&change, &make_account());
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].sender, "user-1");
        assert_eq!(msgs[1].sender, "user-2");
    }

    #[test]
    fn metadata_contains_phone_number_id() {
        let change = make_change(
            vec![WhatsAppMessage {
                from: "user-1".into(),
                id: "msg-1".into(),
                timestamp: "1".into(),
                message_type: "text".into(),
                text: Some(TextMessage { body: "Hi".into() }),
            }],
            vec![],
        );

        let msgs = parse_inbound(&change, &make_account());
        assert_eq!(
            msgs[0]
                .raw_metadata
                .get("phone_number_id")
                .and_then(|v| v.as_str()),
            Some("phone-123")
        );
    }
}
