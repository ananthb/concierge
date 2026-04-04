use serde::{Deserialize, Serialize};

// ============================================================================
// Tenant Types
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tenant {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    #[serde(default)]
    pub facebook_id: Option<String>,
    pub plan: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TenantCredentials {}

// ============================================================================
// WhatsApp Account Resource
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WhatsAppAccount {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub phone_number: String,
    pub phone_number_id: String,
    pub auto_reply: AutoReplyConfig,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AutoReplyConfig {
    pub enabled: bool,
    pub mode: AutoReplyMode,
    pub prompt: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AutoReplyMode {
    #[default]
    Static,
    Ai,
}

// ============================================================================
// Instagram Account Resource
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstagramAccount {
    pub id: String,
    pub tenant_id: String,
    pub instagram_user_id: String,
    pub instagram_username: String,
    pub page_id: String,
    pub auto_reply: AutoReplyConfig,
    pub enabled: bool,
    pub created_at: String,
}

// ============================================================================
// Lead Capture Form
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LeadCaptureForm {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub slug: String,
    pub whatsapp_account_id: String,
    pub reply_mode: AutoReplyMode,
    pub reply_prompt: String,
    pub style: LeadFormStyle,
    pub allowed_origins: Vec<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LeadFormStyle {
    pub primary_color: String,
    pub text_color: String,
    pub background_color: String,
    pub border_radius: String,
    pub button_text: String,
    pub placeholder_text: String,
    pub success_message: String,
    #[serde(default)]
    pub custom_css: String,
}

impl Default for LeadFormStyle {
    fn default() -> Self {
        Self {
            primary_color: String::from("#F38020"),
            text_color: String::from("#333333"),
            background_color: String::from("#ffffff"),
            border_radius: String::from("8px"),
            button_text: String::from("Get in touch"),
            placeholder_text: String::from("Your phone number"),
            success_message: String::from("Thanks! We'll message you on WhatsApp shortly."),
            custom_css: String::new(),
        }
    }
}

// ============================================================================
// Instagram Token
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstagramToken {
    pub access_token: String,
    pub expires_at: String,
    pub user_id: String,
}

// ============================================================================
// WhatsApp Webhook Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct WhatsAppWebhook {
    pub object: String,
    #[serde(default)]
    pub entry: Vec<WebhookEntry>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEntry {
    pub id: String,
    #[serde(default)]
    pub changes: Vec<WebhookChange>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookChange {
    pub field: String,
    pub value: WebhookValue,
}

#[derive(Debug, Deserialize)]
pub struct WebhookValue {
    pub messaging_product: String,
    pub metadata: WebhookMetadata,
    #[serde(default)]
    pub contacts: Vec<WebhookContact>,
    #[serde(default)]
    pub messages: Vec<WhatsAppMessage>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookMetadata {
    pub display_phone_number: String,
    pub phone_number_id: String,
}

#[derive(Debug, Deserialize)]
pub struct WebhookContact {
    pub wa_id: String,
    pub profile: ContactProfile,
}

#[derive(Debug, Deserialize)]
pub struct ContactProfile {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppMessage {
    pub from: String,
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(default)]
    pub text: Option<TextMessage>,
}

#[derive(Debug, Deserialize)]
pub struct TextMessage {
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub from: String,
    pub sender_name: String,
    pub text: String,
    pub message_id: String,
    pub timestamp: String,
}

// ============================================================================
// Instagram DM Webhook Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct InstagramWebhookPayload {
    pub object: String,
    #[serde(default)]
    pub entry: Vec<InstagramWebhookEntry>,
}

#[derive(Debug, Deserialize)]
pub struct InstagramWebhookEntry {
    pub id: String,
    #[serde(default)]
    pub time: i64,
    #[serde(default)]
    pub messaging: Vec<InstagramMessaging>,
}

#[derive(Debug, Deserialize)]
pub struct InstagramMessaging {
    pub sender: IdField,
    pub recipient: IdField,
    #[serde(default)]
    pub timestamp: i64,
    #[serde(default)]
    pub message: Option<InstagramDm>,
}

#[derive(Debug, Deserialize)]
pub struct IdField {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct InstagramDm {
    pub mid: String,
    #[serde(default)]
    pub text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_reply_mode_serialization() {
        assert_eq!(
            serde_json::to_string(&AutoReplyMode::Static).unwrap(),
            "\"static\""
        );
        assert_eq!(serde_json::to_string(&AutoReplyMode::Ai).unwrap(), "\"ai\"");
        let mode: AutoReplyMode = serde_json::from_str("\"ai\"").unwrap();
        assert_eq!(mode, AutoReplyMode::Ai);
    }

    #[test]
    fn test_lead_form_style_default() {
        let style = LeadFormStyle::default();
        assert_eq!(style.primary_color, "#F38020");
        assert_eq!(style.button_text, "Get in touch");
        assert!(style.custom_css.is_empty());
    }

    #[test]
    fn test_whatsapp_webhook_deserialization() {
        let json = r#"{
            "object": "whatsapp_business_account",
            "entry": [{
                "id": "123456789",
                "changes": [{
                    "field": "messages",
                    "value": {
                        "messaging_product": "whatsapp",
                        "metadata": {
                            "display_phone_number": "+1234567890",
                            "phone_number_id": "phone-123"
                        },
                        "contacts": [{
                            "wa_id": "user123",
                            "profile": {"name": "Test User"}
                        }],
                        "messages": [{
                            "from": "user123",
                            "id": "msg-123",
                            "timestamp": "1234567890",
                            "type": "text",
                            "text": {"body": "Hello!"}
                        }]
                    }
                }]
            }]
        }"#;

        let webhook: WhatsAppWebhook = serde_json::from_str(json).unwrap();
        assert_eq!(webhook.object, "whatsapp_business_account");
        assert_eq!(
            webhook.entry[0].changes[0].value.messages[0].from,
            "user123"
        );
    }

    #[test]
    fn test_instagram_webhook_deserialization() {
        let json = r#"{
            "object": "instagram",
            "entry": [{
                "id": "page-123",
                "time": 1700000000,
                "messaging": [{
                    "sender": {"id": "sender-456"},
                    "recipient": {"id": "page-123"},
                    "timestamp": 1700000000,
                    "message": {
                        "mid": "mid-789",
                        "text": "Hello!"
                    }
                }]
            }]
        }"#;

        let payload: InstagramWebhookPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.object, "instagram");
        assert_eq!(payload.entry[0].messaging[0].sender.id, "sender-456");
        assert_eq!(
            payload.entry[0].messaging[0]
                .message
                .as_ref()
                .unwrap()
                .text
                .as_deref(),
            Some("Hello!")
        );
    }
}
