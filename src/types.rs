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
    /// BCP-47 locale tag, e.g. "en-IN", "en-US". Drives UI grouping and
    /// (in Phase 2) translated copy. Currency below is a separate override
    /// that lets a tenant see prices in INR while reading English-IN copy.
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default = "default_currency")]
    pub currency: String,
    /// Each pack adds 5 to the tenant's email-address quota. Quota = 1 + 5 *
    /// this. Packs are purchased one-time via Razorpay.
    #[serde(default)]
    pub email_address_packs_purchased: u32,
    pub created_at: String,
    pub updated_at: String,
}

impl Tenant {
    pub fn email_address_quota(&self) -> u32 {
        1 + 5 * self.email_address_packs_purchased
    }
}

fn default_currency() -> String {
    "INR".to_string()
}

fn default_locale() -> String {
    "en-IN".to_string()
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
    pub auto_reply: ReplyConfig,
    pub created_at: String,
    pub updated_at: String,
}

/// Per-channel reply routing: an ordered list of rules whose first match wins,
/// plus a mandatory default rule that fires when nothing matches.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReplyConfig {
    pub enabled: bool,
    #[serde(default)]
    pub rules: Vec<ReplyRule>,
    pub default_rule: ReplyRule,
    /// Seconds to wait after the latest inbound message before replying.
    /// Lets users finish typing and groups multi-message bursts into one
    /// AI call. 0 = reply immediately (no buffering).
    #[serde(default = "default_wait_seconds")]
    pub wait_seconds: u32,
}

impl Default for ReplyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rules: Vec::new(),
            default_rule: ReplyRule::default_fallback(),
            wait_seconds: default_wait_seconds(),
        }
    }
}

impl ReplyConfig {
    /// Convenience: read the default rule's text without unwrapping the enum.
    /// Used by channel admin templates that still expose a single
    /// "default response" field while a richer rules UI is built out.
    pub fn default_text(&self) -> &str {
        match &self.default_rule.response {
            ReplyResponse::Canned { text } | ReplyResponse::Prompt { text } => text,
        }
    }

    /// True when the default rule sends static text (no LLM, no credit).
    pub fn default_is_canned(&self) -> bool {
        matches!(self.default_rule.response, ReplyResponse::Canned { .. })
    }

    /// Mutate the default rule from an admin form. `mode` is the wire value
    /// from the form ("canned" / "prompt" / legacy "static" / "ai").
    pub fn set_default_response(&mut self, mode: &str, text: String) {
        self.default_rule.response = match mode {
            "ai" | "prompt" => ReplyResponse::Prompt { text },
            _ => ReplyResponse::Canned { text },
        };
    }
}

pub fn default_wait_seconds() -> u32 {
    5
}

/// One reply routing entry: a matcher (when does this fire?) and a response
/// (what do we send?).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReplyRule {
    pub id: String,
    pub label: String,
    pub matcher: ReplyMatcher,
    pub response: ReplyResponse,
}

impl ReplyRule {
    /// The default fallback rule used when a tenant hasn't customized.
    /// Calls the LLM with the persona prompt + a generic instruction.
    pub fn default_fallback() -> Self {
        Self {
            id: "default".to_string(),
            label: "Default reply".to_string(),
            matcher: ReplyMatcher::Default,
            response: ReplyResponse::Prompt {
                text: "Reply to the customer's message helpfully.".to_string(),
            },
        }
    }
}

/// How a rule decides whether it matches the inbound message.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReplyMatcher {
    /// Only valid for the `default_rule` slot. Always matches.
    Default,
    /// Match if any keyword (case-insensitive substring) appears in the message.
    StaticText { keywords: Vec<String> },
    /// Embedding-based intent match. The `embedding` is precomputed from
    /// `description` on save; the pipeline compares it to the embedded
    /// inbound message via cosine similarity.
    Prompt {
        description: String,
        #[serde(default)]
        embedding: Vec<f32>,
        #[serde(default)]
        embedding_model: String,
        #[serde(default = "default_match_threshold")]
        threshold: f32,
    },
}

pub fn default_match_threshold() -> f32 {
    0.72
}

/// What to send when a rule matches.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReplyResponse {
    /// Send this text verbatim. No AI call, no credit.
    Canned { text: String },
    /// Append this prompt to the persona prompt and run the main LLM.
    Prompt { text: String },
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
    pub auto_reply: ReplyConfig,
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
    pub reply: ReplyResponse,
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
// Email Address Types
// ============================================================================

/// One concierge email address owned by a tenant. The full address is
/// `{local_part}@{EMAIL_BASE_DOMAIN}` (the platform's single email domain).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EmailAddress {
    pub local_part: String,
    pub tenant_id: String,
    #[serde(default)]
    pub auto_reply: ReplyConfig,
    #[serde(default)]
    pub notification_recipients: Vec<NotificationRecipient>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NotificationRecipient {
    pub id: String,
    pub address: String,
    pub kind: RecipientKind,
    pub status: RecipientStatus,
    /// True for the tenant owner's auth-login email; auto-verified, can't be
    /// deleted by the user.
    #[serde(default)]
    pub is_owner: bool,
    pub created_at: String,
    #[serde(default)]
    pub verified_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecipientKind {
    Cc,
    Bcc,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecipientStatus {
    Pending,
    Verified,
}

/// Reverse alias mapping for reply routing: when Concierge forwards a
/// message out of the platform, the recipient's `Reply-To` is set to a
/// short-lived alias so their reply lands back here and can be re-routed.
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
#[derive(Serialize, Deserialize, Clone, Debug)]
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
    #[serde(default)]
    pub persona: PersonaConfig,
    /// Default wait_seconds copied into ReplyConfig on every channel account
    /// this tenant connects later. Per-account overrides live on each ReplyConfig.
    #[serde(default = "default_wait_seconds")]
    pub default_wait_seconds: u32,
    pub completed: bool,
}

/// Tenant-wide AI persona used as the system prompt for every AI reply.
/// The persona is one of three sources (Preset, Builder, Custom) — never a
/// mix — so there is exactly one source of truth for the active prompt.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PersonaConfig {
    pub source: PersonaSource,
    #[serde(default)]
    pub safety: PersonaSafety,
}

impl Default for PersonaConfig {
    fn default() -> Self {
        Self {
            source: PersonaSource::Preset(PersonaPreset::FriendlyFlorist),
            safety: PersonaSafety::default(),
        }
    }
}

impl PersonaConfig {
    /// The actual prompt sent to the LLM. Computed from the source on demand.
    pub fn active_prompt(&self) -> String {
        match &self.source {
            PersonaSource::Preset(p) => p.prompt().to_string(),
            PersonaSource::Builder(b) => crate::personas::generate(b),
            PersonaSource::Custom(s) => s.clone(),
        }
    }

    /// SHA-256 of the active prompt, used to detect when a re-run of the
    /// safety classifier is needed.
    pub fn active_prompt_hash(&self) -> String {
        crate::helpers::sha256_hex(&self.active_prompt())
    }

    /// True if AI replies are allowed: the safety check has approved the
    /// current prompt (no hash drift since approval).
    pub fn is_safe_to_use(&self) -> bool {
        matches!(self.safety.status, PersonaSafetyStatus::Approved)
            && self.safety.checked_prompt_hash.as_deref()
                == Some(self.active_prompt_hash().as_str())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PersonaSource {
    /// Wizard default: a curated preset's bundled prompt is used verbatim.
    Preset(PersonaPreset),
    /// User-filled inputs the system uses to compose a prompt on demand.
    Builder(PersonaBuilder),
    /// Power-user override: raw prompt text. Replaces builder/preset entirely.
    Custom(String),
}

/// Curated persona presets shipped in the app. Add a variant here AND in
/// `personas.rs` (label/description/prompt/default_rules) to ship a new one.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PersonaPreset {
    FriendlyFlorist,
    ProfessionalSalon,
    PlayfulCafe,
    OldSchoolClinic,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PersonaBuilder {
    #[serde(default)]
    pub biz_type: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub catch_phrases: Vec<String>,
    #[serde(default)]
    pub off_topics: Vec<String>,
    #[serde(default)]
    pub never: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PersonaSafety {
    #[serde(default)]
    pub status: PersonaSafetyStatus,
    /// SHA-256 of the prompt that was last vetted. Used to detect when the
    /// active prompt has drifted (e.g. user edited but new check hasn't
    /// completed) and AI replies must be paused.
    #[serde(default)]
    pub checked_prompt_hash: Option<String>,
    #[serde(default)]
    pub checked_at: Option<String>,
    /// User-facing decline reason for the Rejected case. Always vague — the
    /// internal classifier category is logged but not exposed so users can't
    /// iterate prompts against the classifier.
    #[serde(default)]
    pub vague_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PersonaSafetyStatus {
    #[default]
    Pending,
    Approved,
    Rejected,
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
    /// Reply when the bot is @mentioned in any channel of the guild.
    #[serde(default)]
    pub inbound_mentions: bool,
    /// Reply to every message in these channels (regardless of mention).
    #[serde(default)]
    pub inbound_channel_ids: Vec<String>,
    /// AI auto-reply configuration for inbound Discord messages.
    #[serde(default)]
    pub auto_reply: ReplyConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reply_response_serialization() {
        let canned = ReplyResponse::Canned {
            text: "hi".to_string(),
        };
        let s = serde_json::to_string(&canned).unwrap();
        assert!(s.contains("\"kind\":\"canned\""));
        assert!(s.contains("\"text\":\"hi\""));
        let prompt = ReplyResponse::Prompt {
            text: "be helpful".to_string(),
        };
        let s = serde_json::to_string(&prompt).unwrap();
        assert!(s.contains("\"kind\":\"prompt\""));
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
// Billing Types: Reply Credits
// ============================================================================

/// Source of a credit entry: determines expiry behavior.
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

/// Tenant billing state: credit ledger with expiry support.
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
