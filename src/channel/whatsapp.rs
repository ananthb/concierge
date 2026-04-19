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
                body: text,
                subject: None,
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

/// Send a reply via WhatsApp.
pub async fn send_reply(
    env: &Env,
    metadata: &serde_json::Value,
    to: &str,
    body: &str,
) -> Result<()> {
    let access_token = env
        .secret("WHATSAPP_ACCESS_TOKEN")
        .map(|s| s.to_string())
        .unwrap_or_default();

    let phone_number_id = metadata
        .get("phone_number_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    crate::whatsapp::send_whatsapp_message(&access_token, phone_number_id, to, body).await
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
            auto_reply: AutoReplyConfig::default(),
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
        assert_eq!(msgs[0].sender_name.as_deref(), Some("Alice"));
        assert_eq!(msgs[0].body, "Hello!");
        assert_eq!(msgs[0].recipient, "phone-123");
        assert_eq!(msgs[0].tenant_id, "tenant-1");
        assert!(msgs[0].subject.is_none());
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
