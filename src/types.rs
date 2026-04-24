use serde::{Deserialize, Serialize};

// ============================================================================
// Tenant Types
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Tenant {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    #[serde(default)]
    pub facebook_id: Option<String>,
    pub plan: String,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub created_at: String,
    pub updated_at: String,
}

fn default_currency() -> String {
    "INR".to_string()
}

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
    #[serde(default)]
    pub updated_at: String,
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

// ============================================================================
// Email Routing Types
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SubdomainStatus {
    Active,
    Suspended,
}

impl Default for SubdomainStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EmailSubdomain {
    pub label: String,
    pub domain: String,
    pub tenant_id: String,
    pub default_action: EmailAction,
    #[serde(default)]
    pub dns_record_ids: Vec<String>,
    #[serde(default)]
    pub subscription_id: Option<String>,
    #[serde(default)]
    pub status: SubdomainStatus,
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RoutingRule {
    pub id: String,
    pub domain: String,
    pub name: String,
    pub priority: i32,
    pub enabled: bool,
    pub criteria: MatchCriteria,
    pub action: EmailAction,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MatchCriteria {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_attachment: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_pattern: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmailAction {
    Drop,
    Spam {
        #[serde(default)]
        message: Option<String>,
    },
    ForwardEmail {
        destination: String,
    },
    ForwardDiscord {
        channel_id: String,
    },
    AiReply {
        #[serde(default)]
        system_prompt: Option<String>,
        #[serde(default)]
        approval_channel_id: Option<String>,
        #[serde(default)]
        approval_email: Option<String>,
    },
}

impl Default for EmailAction {
    fn default() -> Self {
        Self::Drop
    }
}

/// Reverse alias mapping for reply routing.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EmailReverseAlias {
    pub alias: String,
    pub original_sender: String,
    pub tenant_id: String,
    pub domain: String,
}

// ============================================================================
// Unified Messaging Types
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    WhatsApp,
    Instagram,
    Email,
    Discord,
}

impl Channel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::WhatsApp => "whatsapp",
            Channel::Instagram => "instagram",
            Channel::Email => "email",
            Channel::Discord => "discord",
        }
    }
}

/// Unified inbound message from any channel.
#[derive(Clone, Debug)]
pub struct InboundMessage {
    pub id: String,
    pub channel: Channel,
    pub sender: String,
    pub sender_name: Option<String>,
    pub recipient: String,
    pub body: String,
    pub subject: Option<String>,
    pub has_attachment: bool,
    pub tenant_id: String,
    pub channel_account_id: String,
    pub raw_metadata: serde_json::Value,
}

/// Conversation context for cross-channel Discord relay.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConversationContext {
    pub id: String,
    pub discord_message_id: String,
    pub discord_channel_id: String,
    pub origin_channel: Channel,
    pub origin_sender: String,
    pub origin_recipient: String,
    pub tenant_id: String,
    pub channel_account_id: String,
    pub reply_metadata: serde_json::Value,
    #[serde(default)]
    pub ai_draft: Option<String>,
    pub created_at: String,
}

// ============================================================================
// Discord Interaction Types
// ============================================================================

#[derive(Deserialize, Debug)]
pub struct DiscordInteraction {
    pub id: String,
    #[serde(rename = "type")]
    pub interaction_type: u8,
    #[serde(default)]
    pub data: Option<InteractionData>,
    #[serde(default)]
    pub message: Option<serde_json::Value>,
    #[serde(default)]
    pub member: Option<serde_json::Value>,
    pub token: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub channel_id: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct InteractionData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub custom_id: Option<String>,
    #[serde(default)]
    pub options: Option<Vec<CommandOption>>,
    #[serde(default)]
    pub components: Option<Vec<ActionRow>>,
}

#[derive(Deserialize, Debug)]
pub struct CommandOption {
    pub name: String,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    #[serde(default)]
    pub options: Vec<CommandOption>,
}

#[derive(Deserialize, Debug)]
pub struct ActionRow {
    #[serde(default)]
    pub components: Vec<ModalComponent>,
}

#[derive(Deserialize, Debug)]
pub struct ModalComponent {
    #[serde(default)]
    pub custom_id: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
}

/// Business information for KYC / Indian regulatory compliance.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BusinessInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub contact_name: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub business_type: String, // "sole_proprietorship" | "partnership" | "pvt_ltd" | "llp"
    #[serde(default)]
    pub pan: String,
    #[serde(default)]
    pub gstin: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub pincode: String,
}

/// Notification delivery configuration.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NotificationConfig {
    #[serde(default)]
    pub approval_discord: bool,
    #[serde(default)]
    pub approval_email: bool,
    #[serde(default = "default_approval_freq")]
    pub approval_email_frequency_minutes: u32,
    #[serde(default)]
    pub digest_discord: bool,
    #[serde(default)]
    pub digest_email: bool,
    #[serde(default = "default_digest_freq")]
    pub digest_email_frequency_minutes: u32,
}

fn default_approval_freq() -> u32 {
    60
}
fn default_digest_freq() -> u32 {
    1440
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            approval_discord: false,
            approval_email: false,
            approval_email_frequency_minutes: 60,
            digest_discord: false,
            digest_email: false,
            digest_email_frequency_minutes: 1440,
        }
    }
}

/// Onboarding state for the setup wizard.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct OnboardingState {
    pub step: String,
    #[serde(default)]
    pub business: BusinessInfo,
    #[serde(default)]
    pub notifications: NotificationConfig,
    pub persona: PersonaConfig,
    pub canned_replies: Vec<CannedReply>,
    pub completed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PersonaConfig {
    #[serde(default)]
    pub biz_type: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub never: String,
}

impl PersonaConfig {
    pub fn to_system_prompt(&self) -> String {
        let mut parts =
            vec!["You are a helpful messaging assistant for a small business.".to_string()];
        if !self.biz_type.is_empty() {
            let loc = if self.city.is_empty() {
                String::new()
            } else {
                format!(" in {}", self.city)
            };
            parts.push(format!("The business is a {}{}.", self.biz_type, loc));
        }
        if !self.tone.is_empty() {
            parts.push(format!(
                "Tone: {}. Match this tone in every reply.",
                self.tone
            ));
        }
        if !self.never.is_empty() {
            parts.push(format!(
                "Never {}. If asked, politely defer to a human.",
                self.never
            ));
        }
        parts.push(
            "Keep replies short (1-3 sentences). If unsure, hand off to the owner.".to_string(),
        );
        parts.join("\n")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CannedReply {
    pub trigger: String,
    pub reply: String,
}

/// Discord guild → tenant mapping config.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DiscordConfig {
    pub guild_id: String,
    pub tenant_id: String,
    #[serde(default)]
    pub guild_name: Option<String>,
    #[serde(default)]
    pub approval_channel_id: Option<String>,
    #[serde(default)]
    pub digest_channel_id: Option<String>,
    #[serde(default)]
    pub relay_channel_id: Option<String>,
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
    fn test_channel_serialization() {
        assert_eq!(
            serde_json::to_string(&Channel::WhatsApp).unwrap(),
            "\"whats_app\""
        );
        assert_eq!(serde_json::to_string(&Channel::Email).unwrap(), "\"email\"");
        let ch: Channel = serde_json::from_str("\"instagram\"").unwrap();
        assert_eq!(ch, Channel::Instagram);
    }

    #[test]
    fn test_email_action_serialization() {
        let action = EmailAction::ForwardDiscord {
            channel_id: "123".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"forward_discord\""));
        assert!(json.contains("\"channel_id\":\"123\""));

        let parsed: EmailAction = serde_json::from_str(&json).unwrap();
        match parsed {
            EmailAction::ForwardDiscord { channel_id } => assert_eq!(channel_id, "123"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_conversation_context_roundtrip() {
        let ctx = ConversationContext {
            id: "ctx-1".into(),
            discord_message_id: "msg-1".into(),
            discord_channel_id: "ch-1".into(),
            origin_channel: Channel::Email,
            origin_sender: "alice@example.com".into(),
            origin_recipient: "support@proxy.com".into(),
            tenant_id: "tenant-1".into(),
            channel_account_id: "example.com".into(),
            reply_metadata: serde_json::json!({"domain": "example.com"}),
            ai_draft: Some("Draft reply text".into()),
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: ConversationContext = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.origin_channel, Channel::Email);
        assert_eq!(parsed.ai_draft.as_deref(), Some("Draft reply text"));
    }

    #[test]
    fn test_discord_interaction_deserialization() {
        let json = r#"{
            "id": "int-1",
            "type": 2,
            "token": "tok",
            "guild_id": "guild-1",
            "channel_id": "ch-1",
            "data": {
                "name": "status",
                "options": []
            }
        }"#;
        let interaction: DiscordInteraction = serde_json::from_str(json).unwrap();
        assert_eq!(interaction.interaction_type, 2);
        assert_eq!(
            interaction.data.as_ref().unwrap().name.as_deref(),
            Some("status")
        );
        assert_eq!(interaction.guild_id.as_deref(), Some("guild-1"));
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

// ============================================================================
// Billing Types — Reply Credits
// ============================================================================

/// Source of a credit entry — determines expiry behavior.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CreditSource {
    FreeMonthly,
    Purchase,
    Grant,
}

/// A single credit ledger entry with optional expiry.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreditEntry {
    pub amount: i64,
    pub source: CreditSource,
    pub expires_at: Option<String>, // ISO 8601, None = never expires
    pub granted_at: String,         // ISO 8601
}

/// Tenant billing state — credit ledger with expiry support.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TenantBilling {
    #[serde(default)]
    pub credits: Vec<CreditEntry>,
    #[serde(default)]
    pub free_month: Option<String>, // "2026-04" = last month free credits were issued
    #[serde(default)]
    pub replies_used: i64, // lifetime replies sent
}

impl TenantBilling {
    pub fn has_credits(&self) -> bool {
        self.total_remaining() > 0
    }

    pub fn total_remaining(&self) -> i64 {
        self.credits.iter().map(|e| e.amount).sum()
    }
}
