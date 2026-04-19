use worker::*;

use crate::crypto;
use crate::helpers::generate_id;
use crate::instagram;
use crate::storage;
use crate::types::*;

/// Parse Instagram webhook messaging entry into an InboundMessage.
pub fn parse_inbound(
    messaging: &InstagramMessaging,
    account: &InstagramAccount,
    page_id: &str,
) -> Option<InboundMessage> {
    let sender_id = &messaging.sender.id;

    // Skip messages from ourselves
    if sender_id == page_id {
        return None;
    }

    let dm = messaging.message.as_ref()?;
    let text = dm.text.as_ref()?;

    Some(InboundMessage {
        id: generate_id(),
        channel: Channel::Instagram,
        sender: sender_id.clone(),
        sender_name: None,
        recipient: page_id.to_string(),
        body: text.clone(),
        subject: None,
        has_attachment: false,
        tenant_id: account.tenant_id.clone(),
        channel_account_id: account.id.clone(),
        raw_metadata: serde_json::json!({
            "page_id": page_id,
            "instagram_account_id": account.id,
            "message_mid": dm.mid,
        }),
    })
}

/// Send a reply via Instagram DM.
/// Loads the encrypted token from KV, decrypts, and sends.
pub async fn send_reply(
    env: &Env,
    metadata: &serde_json::Value,
    to: &str,
    body: &str,
) -> Result<()> {
    let kv = env.kv("KV")?;
    let encryption_key = env
        .secret("ENCRYPTION_KEY")
        .map(|s| s.to_string())
        .unwrap_or_default();

    let account_id = metadata
        .get("instagram_account_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let page_id = metadata
        .get("page_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Load and decrypt token
    let token_key = format!("instagram_token:{account_id}");
    let encrypted_token = match kv
        .get(&token_key)
        .text()
        .await
        .map_err(|e| Error::from(e.to_string()))?
    {
        Some(t) => t,
        None => return Err(Error::from("Instagram token not found")),
    };

    let token = crypto::decrypt_token(&encrypted_token, &encryption_key).await?;

    if instagram::token_is_expired(&token) {
        return Err(Error::from("Instagram token expired"));
    }

    instagram::send_instagram_dm(&token.access_token, page_id, to, body).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_account() -> InstagramAccount {
        InstagramAccount {
            id: "ig-acc-1".into(),
            tenant_id: "tenant-1".into(),
            instagram_user_id: "ig-user-1".into(),
            instagram_username: "testuser".into(),
            page_id: "page-123".into(),
            auto_reply: AutoReplyConfig::default(),
            enabled: true,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn make_messaging(sender: &str, text: Option<&str>) -> InstagramMessaging {
        InstagramMessaging {
            sender: IdField {
                id: sender.to_string(),
            },
            recipient: IdField {
                id: "page-123".into(),
            },
            timestamp: 1700000000,
            message: text.map(|t| InstagramDm {
                mid: "mid-1".into(),
                text: Some(t.to_string()),
            }),
        }
    }

    #[test]
    fn parse_text_dm() {
        let messaging = make_messaging("sender-456", Some("Hello!"));
        let msg = parse_inbound(&messaging, &make_account(), "page-123");

        let msg = msg.expect("should parse");
        assert_eq!(msg.channel, Channel::Instagram);
        assert_eq!(msg.sender, "sender-456");
        assert_eq!(msg.body, "Hello!");
        assert_eq!(msg.recipient, "page-123");
        assert_eq!(msg.tenant_id, "tenant-1");
    }

    #[test]
    fn skip_self_messages() {
        let messaging = make_messaging("page-123", Some("Echo"));
        let msg = parse_inbound(&messaging, &make_account(), "page-123");
        assert!(msg.is_none());
    }

    #[test]
    fn skip_no_text() {
        let messaging = make_messaging("sender-456", None);
        let msg = parse_inbound(&messaging, &make_account(), "page-123");
        assert!(msg.is_none());
    }

    #[test]
    fn skip_no_message() {
        let messaging = InstagramMessaging {
            sender: IdField {
                id: "sender-456".into(),
            },
            recipient: IdField {
                id: "page-123".into(),
            },
            timestamp: 0,
            message: None,
        };
        let msg = parse_inbound(&messaging, &make_account(), "page-123");
        assert!(msg.is_none());
    }

    #[test]
    fn metadata_contains_page_id() {
        let messaging = make_messaging("sender-456", Some("Hi"));
        let msg = parse_inbound(&messaging, &make_account(), "page-123").unwrap();
        assert_eq!(
            msg.raw_metadata.get("page_id").and_then(|v| v.as_str()),
            Some("page-123")
        );
    }
}
