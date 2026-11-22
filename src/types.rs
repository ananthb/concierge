use serde::{Deserialize, Serialize};

use crate::locale::Currency;

// ============================================================================
// Tenant Types
// ============================================================================

/// Tenant subscription tier.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    /// Complimentary account: usage is tracked (`replies_used` increments)
    /// without blocking on a zero balance.
    Free,
    /// Standard prepaid pay-per-reply account.
    #[default]
    Paid,
}

impl Plan {
    pub fn as_str(self) -> &'static str {
        match self {
            Plan::Free => "free",
            Plan::Paid => "paid",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "free" => Some(Plan::Free),
            "paid" => Some(Plan::Paid),
            _ => None,
        }
    }

    /// Whether the credit balance gates outbound replies.
    pub fn is_metered(self) -> bool {
        matches!(self, Plan::Paid)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Tenant {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    #[serde(default)]
    pub facebook_id: Option<String>,
    #[serde(default)]
    pub plan: Plan,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub currency: Currency,
    pub created_at: String,
    pub updated_at: String,
}

fn default_locale() -> String {
    "en-IN".to_string()
}

// ============================================================================
// Content Safety State
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SafetyStatus {
    #[default]
    Pending,
    Approved,
    Rejected,
}

impl SafetyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SafetyStatus::Pending => "pending",
            SafetyStatus::Approved => "approved",
            SafetyStatus::Rejected => "rejected",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SafetyState {
    #[serde(default)]
    pub status: SafetyStatus,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub checked_at: Option<String>,
    #[serde(default)]
    pub vague_reason: Option<String>,
}

impl SafetyState {
    /// True iff safety has approved the exact current content (no hash drift).
    pub fn is_approved_for(&self, content: &str) -> bool {
        matches!(self.status, SafetyStatus::Approved)
            && self.content_hash.as_deref() == Some(crate::helpers::sha256_hex(content).as_str())
    }
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
    #[serde(default)]
    pub waba_id: Option<String>,
    pub auto_reply: ReplyConfig,
    #[serde(default)]
    pub canned_safety: SafetyState,
    pub created_at: String,
    pub updated_at: String,
}

/// Per-channel reply configuration.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReplyConfig {
    pub enabled: bool,
    #[serde(default = "default_response")]
    pub response: ReplyResponse,
    /// Seconds to wait after the latest inbound message before replying.
    /// 0 = reply immediately (no buffering).
    #[serde(default = "default_wait_seconds")]
    pub wait_seconds: u32,
}

impl Default for ReplyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            response: default_response(),
            wait_seconds: default_wait_seconds(),
        }
    }
}

impl ReplyConfig {
    /// True when the response is static text (`Canned`).
    pub fn is_canned(&self) -> bool {
        matches!(self.response, ReplyResponse::Canned { .. })
    }

    /// Canned text when `Canned`, empty string when `Ai`.
    pub fn canned_text(&self) -> &str {
        match &self.response {
            ReplyResponse::Ai => "",
            ReplyResponse::Canned { text } => text,
        }
    }
}

pub fn default_response() -> ReplyResponse {
    ReplyResponse::Ai
}

pub fn default_wait_seconds() -> u32 {
    3
}

/// What to send when an inbound message arrives.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ReplyResponse {
    /// Use the single small-business core persona via Workers AI.
    Ai,
    /// Send this static text verbatim. Costs 1 reply credit.
    Canned { text: String },
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub messaging_product: String,
    pub metadata: WebhookMetadata,
    #[serde(default)]
    pub contacts: Vec<WebhookContact>,
    #[serde(default)]
    pub messages: Vec<WhatsAppMessage>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookMetadata {
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub timestamp: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub message_type: String,
    #[serde(default)]
    pub text: Option<TextMessage>,
}

#[derive(Debug, Deserialize)]
pub struct TextMessage {
    pub body: String,
}

// ============================================================================
// Unified Messaging Types
// ============================================================================

/// Inbound messaging channel.
///
/// WhatsApp-only today; `Discord` and `Telegram` are planned extensions that
/// plug in by adding a variant, a `channel/` adapter, and a KV lookup arm in
/// `storage::get_channel_reply_config`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    WhatsApp,
}

impl Channel {
    /// Lowercase wire form used by the D1 `channel` column and inside KV keys.
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::WhatsApp => "whatsapp",
        }
    }

    /// Display label used in notifications.
    pub fn label(&self) -> &'static str {
        match self {
            Channel::WhatsApp => "WhatsApp",
        }
    }
}

/// Direction of a row in the unified `messages` table.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageDirection {
    Inbound,
    Outbound,
}

impl MessageDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            MessageDirection::Inbound => "inbound",
            MessageDirection::Outbound => "outbound",
        }
    }
}

/// Outcome recorded on a `messages` row.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageAction {
    /// Auto-reply sent.
    AutoReply,
    /// Draft tripped the risk gate: holding sentence sent and tenant paged.
    HandedOff,
    /// Reply suppressed because the tenant had no remaining reply credits.
    NoCredits,
}

impl MessageAction {
    pub fn as_str(self) -> &'static str {
        match self {
            MessageAction::AutoReply => "auto_reply",
            MessageAction::HandedOff => "handed_off",
            MessageAction::NoCredits => "no_credits",
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
    /// Destination identifier for replies (phone number for WhatsApp; decoupled
    /// so Discord `channel_id` / Telegram `chat_id` fit without pipeline changes).
    #[serde(default)]
    pub reply_to: String,
    pub body: String,
    pub has_attachment: bool,
    pub tenant_id: String,
    pub channel_account_id: String,
    pub raw_metadata: serde_json::Value,
}

impl InboundMessage {
    /// Conversation routing key for this inbound message.
    /// For WhatsApp this is `reply_to` (the customer's `sender` number).
    pub fn conversation_key(&self) -> String {
        if self.reply_to.is_empty() {
            self.sender.clone()
        } else {
            self.reply_to.clone()
        }
    }
}

/// Per-customer conversation session.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Session {
    #[serde(default)]
    pub last_inbound_at: String,
    #[serde(default)]
    pub handoff: Option<HandoffState>,
    #[serde(default)]
    pub conversation_id: String,
    #[serde(default)]
    pub messages: Vec<ConversationMessage>,
}

/// One turn inside a `Session.messages` list.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ConversationMessage {
    pub role: ConversationRole,
    pub content: String,
    pub at: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConversationRole {
    User,
    Assistant,
}

impl ConversationRole {
    pub fn as_wire(self) -> &'static str {
        match self {
            ConversationRole::User => "user",
            ConversationRole::Assistant => "assistant",
        }
    }
}

/// Per-conversation handoff state. Lives inside `Session::handoff`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HandoffState {
    pub signaled_at: String,
    #[serde(default)]
    pub notified: bool,
}

/// Per-tenant overrides for the conversation timing knobs.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ConversationConfig {
    #[serde(default)]
    pub idle_gap_mins: Option<u32>,
    #[serde(default)]
    pub handoff_cooldown_mins: Option<u32>,
    #[serde(default)]
    pub max_history_messages: Option<u32>,
}

impl ConversationConfig {
    pub fn resolve(&self) -> ConversationWindow {
        ConversationWindow {
            idle_gap_mins: self
                .idle_gap_mins
                .map(|v| v as i64)
                .unwrap_or(crate::prompt::DEFAULT_CONVERSATION_IDLE_GAP_MINS),
            handoff_cooldown_mins: self
                .handoff_cooldown_mins
                .map(|v| v as i64)
                .unwrap_or(crate::prompt::DEFAULT_HANDOFF_COOLDOWN_MINS),
            max_history_messages: self
                .max_history_messages
                .unwrap_or(crate::prompt::DEFAULT_CONVERSATION_MAX_MESSAGES),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversationWindow {
    pub idle_gap_mins: i64,
    pub handoff_cooldown_mins: i64,
    pub max_history_messages: u32,
}

/// Business information collected at onboarding.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BusinessInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub contact_name: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub business_type: String,
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

/// Where a human handoff is announced.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NotificationConfig {
    #[serde(default = "default_true", alias = "approval_email")]
    pub handoff_email: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            handoff_email: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Steps in the onboarding wizard, in display order.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingStep {
    #[default]
    Basics,
    Channels,
    Launch,
}

impl OnboardingStep {
    pub const ALL: [OnboardingStep; 3] = [
        OnboardingStep::Basics,
        OnboardingStep::Channels,
        OnboardingStep::Launch,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            OnboardingStep::Basics => "basics",
            OnboardingStep::Channels => "channels",
            OnboardingStep::Launch => "launch",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "basics" => Some(OnboardingStep::Basics),
            "channels" => Some(OnboardingStep::Channels),
            "launch" => Some(OnboardingStep::Launch),
            _ => None,
        }
    }

    pub fn index(self) -> usize {
        match self {
            OnboardingStep::Basics => 0,
            OnboardingStep::Channels => 1,
            OnboardingStep::Launch => 2,
        }
    }
}

/// Onboarding state for the setup wizard.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OnboardingState {
    #[serde(default)]
    pub step: OnboardingStep,
    #[serde(default)]
    pub business: BusinessInfo,
    #[serde(default)]
    pub notifications: NotificationConfig,
    #[serde(default)]
    pub conversation: ConversationConfig,
    #[serde(default)]
    pub profile_safety: SafetyState,
    #[serde(default = "default_wait_seconds")]
    pub default_wait_seconds: u32,
    #[serde(default)]
    pub completed: bool,
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self {
            step: OnboardingStep::default(),
            business: BusinessInfo::default(),
            notifications: NotificationConfig::default(),
            conversation: ConversationConfig::default(),
            profile_safety: SafetyState::default(),
            default_wait_seconds: default_wait_seconds(),
            completed: false,
        }
    }
}

// ============================================================================
// Billing Types: Reply Credits
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CreditSource {
    Purchase,
    Grant,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreditEntry {
    pub amount: i64,
    pub source: CreditSource,
    pub expires_at: Option<String>,
    pub granted_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TenantBilling {
    #[serde(default)]
    pub credits: Vec<CreditEntry>,
    #[serde(default)]
    pub replies_used: i64,
}

impl TenantBilling {
    pub fn has_credits(&self) -> bool {
        self.total_remaining() > 0
    }

    pub fn total_remaining(&self) -> i64 {
        self.credits.iter().map(|e| e.amount).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onboarding_step_round_trip_and_index() {
        for step in OnboardingStep::ALL {
            assert_eq!(OnboardingStep::from_wire(step.as_str()), Some(step));
        }
        assert_eq!(OnboardingStep::from_wire("persona"), None);
        for (i, step) in OnboardingStep::ALL.iter().enumerate() {
            assert_eq!(step.index(), i);
        }
    }

    #[test]
    fn plan_from_wire_rejects_unknown() {
        assert_eq!(Plan::from_wire("free"), Some(Plan::Free));
        assert_eq!(Plan::from_wire("paid"), Some(Plan::Paid));
        assert_eq!(Plan::from_wire("enterprise"), None);
    }

    #[test]
    fn plan_is_metered_only_for_paid() {
        assert!(Plan::Paid.is_metered());
        assert!(!Plan::Free.is_metered());
    }

    #[test]
    fn test_reply_response_serialization() {
        let canned = ReplyResponse::Canned {
            text: "hi".to_string(),
        };
        let s = serde_json::to_string(&canned).unwrap();
        assert!(s.contains("\"mode\":\"canned\""));
        assert!(s.contains("\"text\":\"hi\""));

        let ai = ReplyResponse::Ai;
        let s = serde_json::to_string(&ai).unwrap();
        assert_eq!(s, "{\"mode\":\"ai\"}");
    }

    #[test]
    fn channel_serde_and_wire_strings_match() {
        assert_eq!(Channel::WhatsApp.as_str(), "whatsapp");
        assert_eq!(
            serde_json::to_string(&Channel::WhatsApp).unwrap(),
            "\"whatsapp\""
        );
        let parsed: Channel = serde_json::from_str("\"whatsapp\"").unwrap();
        assert_eq!(parsed, Channel::WhatsApp);
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
    fn conversation_config_resolve_uses_defaults_when_unset() {
        let cfg = ConversationConfig::default();
        let win = cfg.resolve();
        assert_eq!(
            win.idle_gap_mins,
            crate::prompt::DEFAULT_CONVERSATION_IDLE_GAP_MINS
        );
        assert_eq!(
            win.handoff_cooldown_mins,
            crate::prompt::DEFAULT_HANDOFF_COOLDOWN_MINS
        );
        assert_eq!(
            win.max_history_messages,
            crate::prompt::DEFAULT_CONVERSATION_MAX_MESSAGES
        );
    }

    #[test]
    fn conversation_config_resolve_honors_overrides() {
        let cfg = ConversationConfig {
            idle_gap_mins: Some(30),
            handoff_cooldown_mins: Some(15),
            max_history_messages: Some(8),
        };
        let win = cfg.resolve();
        assert_eq!(win.idle_gap_mins, 30);
        assert_eq!(win.handoff_cooldown_mins, 15);
        assert_eq!(win.max_history_messages, 8);
    }

    #[test]
    fn session_round_trips_with_messages_and_conversation_id() {
        let session = Session {
            last_inbound_at: "2026-05-04T12:00:00.000Z".to_string(),
            handoff: None,
            conversation_id: "conv-abc".to_string(),
            messages: vec![
                ConversationMessage {
                    role: ConversationRole::User,
                    content: "hi there".to_string(),
                    at: "2026-05-04T12:00:00.000Z".to_string(),
                },
                ConversationMessage {
                    role: ConversationRole::Assistant,
                    content: "hello!".to_string(),
                    at: "2026-05-04T12:00:01.000Z".to_string(),
                },
            ],
        };
        let s = serde_json::to_string(&session).unwrap();
        let back: Session = serde_json::from_str(&s).unwrap();
        assert_eq!(back.conversation_id, "conv-abc");
        assert_eq!(back.messages.len(), 2);
        assert_eq!(back.messages[0].role, ConversationRole::User);
        assert_eq!(back.messages[1].role, ConversationRole::Assistant);
    }
}
