use serde::{Deserialize, Serialize};

use crate::locale::Currency;

// ============================================================================
// Tenant Types
// ============================================================================

/// Tenant subscription tier. Pricing surfaces (`templates/management.rs`)
/// match on this enum; rate-limit/quota logic that needs a plan branch
/// adds a method here so adding a tier doesn't fan out across files.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    #[default]
    Free,
    Starter,
    Pro,
    Business,
}

impl Plan {
    pub const ALL: &'static [Plan] = &[Plan::Free, Plan::Starter, Plan::Pro, Plan::Business];

    pub fn as_str(self) -> &'static str {
        match self {
            Plan::Free => "free",
            Plan::Starter => "starter",
            Plan::Pro => "pro",
            Plan::Business => "business",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Plan::Free => "Free",
            Plan::Starter => "Starter",
            Plan::Pro => "Pro",
            Plan::Business => "Business",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "free" => Some(Plan::Free),
            "starter" => Some(Plan::Starter),
            "pro" => Some(Plan::Pro),
            "business" => Some(Plan::Business),
            _ => None,
        }
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
    /// BCP-47 locale tag, e.g. "en-IN", "en-US". Drives UI grouping and
    /// (in Phase 2) translated copy. Currency below is a separate override
    /// that lets a tenant see prices in INR while reading English-IN copy.
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub currency: Currency,
    /// Cumulative reply-email-address quota the tenant has purchased.
    /// Each successful pack purchase bumps this by `email_pack_size`
    /// (default 5). Quota = this value (no implicit freebie). Pricing and
    /// pack size live in the singleton `pricing_config` row.
    #[serde(default)]
    pub email_address_extras_purchased: u32,
    /// Set the first time we observe a captured Razorpay payment for this
    /// tenant. The sign-up wizard charges a small refundable verification
    /// amount; this flips on success and gates wizard "Finish".
    #[serde(default)]
    pub verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Tenant {
    /// How many reply-email addresses this tenant is allowed to provision.
    /// Equal to the cumulative count of addresses granted by paid pack
    /// purchases. (Operator grants and pack-purchase webhooks both bump
    /// `email_address_extras_purchased` — see billing/webhook.rs.)
    pub fn email_address_quota(&self) -> u32 {
        self.email_address_extras_purchased
    }
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
    #[serde(default)]
    pub approval: ApprovalPolicy,
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
            approval: ApprovalPolicy::default(),
        }
    }
}

/// Per-rule policy for AI-generated drafts. Only consulted when the rule's
/// `response` is `ReplyResponse::Prompt` (canned rules send verbatim, no draft).
///
/// `Auto` is the default and runs the cheap risk gate: drafts that look
/// risky get queued for human approval, the rest send. `Always` queues every
/// AI draft. `NoGate` skips the safety check entirely and is locked behind
/// the operator's `ALLOW_NO_GATE` env var plus a per-rule TOS acceptance.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApprovalPolicy {
    #[default]
    Auto,
    Always,
    NoGate {
        acceptance: NoGateAcceptance,
    },
}

/// TOS acceptance recorded when a tenant flips a rule into `NoGate`. Lives
/// inside the variant so changing the policy drops the acceptance.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct NoGateAcceptance {
    pub accepted_at: String,
    pub accepted_by: String,
    /// Wording version. Bump when the disclaimer text materially changes,
    /// so we know whether existing acceptances cover the new copy.
    pub version: String,
}

/// Why an AI draft was diverted from the auto-send path to the approval
/// queue. Logged on the pending_approvals row and surfaced in the admin UI.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueueReason {
    /// Rule policy is `Always`. No risk-gate signal, just the explicit choice.
    RuleAlways,
    /// Draft was very short or very long.
    RiskLength,
    /// Draft mentioned money, prices, or refunds.
    RiskMoneyWord,
    /// Draft made a commitment (guarantee/promise/by-day).
    RiskCommitment,
    /// Draft contained a topic in the persona's off-topics or never list.
    RiskPersonaDrift,
}

/// One row of the pending_approvals D1 table, mirrored as a Rust struct.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PendingApproval {
    pub id: String,
    pub tenant_id: String,
    pub channel: Channel,
    pub channel_account_id: String,
    pub rule_id: String,
    pub rule_label: String,
    pub sender: String,
    pub sender_name: Option<String>,
    pub inbound_preview: String,
    pub draft: String,
    pub queue_reason: QueueReason,
    pub status: ApprovalStatus,
    pub created_at: String,
    pub decided_at: Option<String>,
    pub decided_by: Option<String>,
    pub edited: bool,
    pub last_digest_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

/// Who decided a pending approval. Stored on `pending_approvals.decided_by`
/// in a flat string form (`"discord:<id>" | "web:<email>" | "expired"`) so
/// the column stays human-readable in the audit log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApprovalDecider {
    Discord { user_id: String },
    Web { email: String },
    Expired,
}

impl ApprovalDecider {
    /// Wire form for the `decided_by` column.
    pub fn wire(&self) -> String {
        match self {
            ApprovalDecider::Discord { user_id } => format!("discord:{user_id}"),
            ApprovalDecider::Web { email } => format!("web:{email}"),
            ApprovalDecider::Expired => "expired".to_string(),
        }
    }

    /// Inverse of `wire`: parse a stored value. Returns `None` for unknown
    /// forms. Used by tests and any future read path that needs to branch
    /// on who decided.
    pub fn from_wire(s: &str) -> Option<Self> {
        if s == "expired" {
            return Some(ApprovalDecider::Expired);
        }
        if let Some(id) = s.strip_prefix("discord:") {
            return Some(ApprovalDecider::Discord {
                user_id: id.to_string(),
            });
        }
        if let Some(email) = s.strip_prefix("web:") {
            return Some(ApprovalDecider::Web {
                email: email.to_string(),
            });
        }
        None
    }
}

/// How a rule decides whether it matches the inbound message.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReplyMatcher {
    /// Only valid for the `default_rule` slot. Always matches.
    Default,
    /// Match if any keyword (case-insensitive substring) appears in the message.
    Keyword { keywords: Vec<String> },
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
/// `{local_part}@{EMAIL_DOMAIN}` (the platform's single email domain).
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
    /// Lowercase wire form used by the D1 `channel` column and inside
    /// API/webhook payloads. Diverges from serde's snake_case form (which
    /// would emit "whats_app"); keep both intact.
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::WhatsApp => "whatsapp",
            Channel::Instagram => "instagram",
            Channel::Email => "email",
            Channel::Discord => "discord",
        }
    }

    /// Display label used in templates and email digests.
    pub fn label(&self) -> &'static str {
        match self {
            Channel::WhatsApp => "WhatsApp",
            Channel::Instagram => "Instagram",
            Channel::Email => "Email",
            Channel::Discord => "Discord",
        }
    }
}

/// Direction of a row in the unified `messages` table.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageDirection {
    Inbound,
    Outbound,
    /// A message routed back through Discord cross-channel relay.
    Relay,
}

impl MessageDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            MessageDirection::Inbound => "inbound",
            MessageDirection::Outbound => "outbound",
            MessageDirection::Relay => "relay",
        }
    }
}

/// What was done with a message after the pipeline routed it. Stored on
/// the `messages` row so an operator can audit which path fired.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageAction {
    /// Auto-reply sent without any approval gate.
    AutoReply,
    /// Forwarded to Discord for human relay (not an AI draft).
    Relay,
    /// AI draft was diverted to the approval queue.
    AiQueued,
    /// AI draft sent after a human approval (Discord button or web).
    AiApproved,
    /// AI draft rejected; credit refunded.
    AiRejected,
    /// AI draft expired past its 24h hold without action.
    AiExpired,
}

impl MessageAction {
    pub fn as_str(self) -> &'static str {
        match self {
            MessageAction::AutoReply => "auto_reply",
            MessageAction::Relay => "relay",
            MessageAction::AiQueued => "ai_queued",
            MessageAction::AiApproved => "ai_approved",
            MessageAction::AiRejected => "ai_rejected",
            MessageAction::AiExpired => "ai_expired",
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
///
/// Doubles as the per-conversation state store for human-handoff: when
/// the AI emits the handoff token in a reply, the pipeline records
/// `handoff_signaled_at` here so subsequent customer messages on the
/// same conversation route through the holding-pattern prompt instead
/// of the persona — and stop replying entirely after the tenant's
/// effective handoff cooldown (see
/// [`crate::prompt::DEFAULT_HANDOFF_COOLDOWN_MINS`]).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConversationContext {
    pub id: String,
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
    /// RFC3339 timestamp of the *first* turn on this conversation that
    /// emitted the handoff token. Once set, every subsequent turn
    /// follows the holding-pattern path until the cooldown expires.
    #[serde(default)]
    pub handoff_signaled_at: Option<String>,
    /// One-shot guard: flips to true once the tenant has been alerted
    /// (Discord + email) so additional customer turns inside the
    /// holding-pattern window don't re-spam them.
    #[serde(default)]
    pub handoff_notified: bool,
}

/// Per-customer conversation session. Stable-keyed by
/// `(tenant_id, channel, channel_account_id, sender)` (see
/// `storage::save_session`).
///
/// Concierge doesn't have a formal "thread" object — a Session is the
/// soft equivalent. A conversation is considered ended once the
/// customer has been silent for the tenant's effective idle gap (see
/// [`crate::prompt::DEFAULT_CONVERSATION_IDLE_GAP_MINS`] and
/// [`ConversationConfig`]): the next inbound after that gap starts a
/// fresh conversation (any in-progress handoff state is wiped, and
/// the message history is cleared). Within the gap, all inbound from
/// the same sender belong to the same conversation regardless of
/// length.
///
/// This is distinct from `ConversationContext` (which is per
/// AI-draft-approval). Sessions are per-thread.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Session {
    /// RFC3339 timestamp of the most recent inbound message we
    /// processed for this thread. Updated on every inbound, including
    /// during the post-cooldown silent window — so as long as the
    /// customer keeps pinging within the idle gap, the thread stays
    /// bound to the same conversation (and the same handoff record,
    /// if any).
    #[serde(default)]
    pub last_inbound_at: String,
    /// In-progress handoff sub-state. `None` for a normal conversation;
    /// populated once the AI signals a handoff. Cleared at conversation
    /// boundaries (idle gap exceeded).
    #[serde(default)]
    pub handoff: Option<HandoffState>,
    /// Stable id for the active conversation. Bumped when the idle-gap
    /// fires and a fresh conversation starts. Stamped onto every D1
    /// `messages` row written for this thread so an audit log can
    /// reconstruct who said what to whom in which conversation.
    #[serde(default)]
    pub conversation_id: String,
    /// Bounded list of recent turns within this conversation,
    /// forming the chat context handed back to the model on each
    /// turn. Capped to the tenant's effective `max_history_messages`.
    /// Cleared when a fresh conversation starts.
    #[serde(default)]
    pub messages: Vec<ConversationMessage>,
}

/// One turn inside a `Session.messages` list. Roles mirror what
/// Workers AI expects in its `messages` array — `User` for inbound
/// customer text, `Assistant` for outbound replies we actually sent.
/// Drafts queued for approval are NOT recorded here: only sent
/// outbounds become history.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ConversationMessage {
    pub role: ConversationRole,
    pub content: String,
    /// RFC3339 timestamp of the turn. Useful for audit/diagnostics;
    /// not currently used by the AI call itself.
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
    /// RFC3339 timestamp of the AI turn that emitted the handoff
    /// token. Used to compute the cooldown window (see
    /// [`crate::prompt::DEFAULT_HANDOFF_COOLDOWN_MINS`] and the
    /// per-tenant override in [`ConversationConfig`]).
    pub signaled_at: String,
    /// One-shot guard so additional customer messages inside the
    /// holding-pattern window don't re-page the tenant.
    #[serde(default)]
    pub notified: bool,
}

/// Per-tenant overrides for the conversation timing knobs. All fields
/// are `Option`: a `None` means "use the global default" from
/// `prompt::DEFAULT_*`. Lives on `OnboardingState`. Pipeline collapses
/// the optionals into concrete numbers via [`ConversationConfig::resolve`].
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ConversationConfig {
    /// How long the customer can stay silent before the next inbound
    /// is treated as a fresh conversation. Falls back to
    /// [`crate::prompt::DEFAULT_CONVERSATION_IDLE_GAP_MINS`].
    #[serde(default)]
    pub idle_gap_mins: Option<u32>,
    /// How long after a handoff signal we keep replying with the
    /// holding-pattern voice before going silent. Falls back to
    /// [`crate::prompt::DEFAULT_HANDOFF_COOLDOWN_MINS`].
    #[serde(default)]
    pub handoff_cooldown_mins: Option<u32>,
    /// How many recent turns we keep as chat context for the AI.
    /// Falls back to
    /// [`crate::prompt::DEFAULT_CONVERSATION_MAX_MESSAGES`].
    #[serde(default)]
    pub max_history_messages: Option<u32>,
}

impl ConversationConfig {
    /// Collapse the optional per-tenant overrides into concrete
    /// numbers, falling back to the `prompt::DEFAULT_*` constants for
    /// any field the tenant has not overridden. Pure: no KV/D1 reads.
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

/// Concrete per-turn conversation window. Output of
/// [`ConversationConfig::resolve`]; consumed by the pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversationWindow {
    pub idle_gap_mins: i64,
    pub handoff_cooldown_mins: i64,
    pub max_history_messages: u32,
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

/// Notification delivery configuration. Today this only covers approval
/// notifications. The previous activity-summary digest scaffolding was
/// dropped: it was wizard-collected but never read by any send path.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct NotificationConfig {
    #[serde(default)]
    pub approval_discord: bool,
    #[serde(default)]
    pub approval_email: bool,
    #[serde(default)]
    pub approval_email_cadence: DigestCadence,
}

/// How often a tenant wants the approval-queue digest email. The cron sweep
/// runs every 15 minutes and skips tenants whose cadence isn't due yet.
/// `Instant` means a single-item email per draft, no batching.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DigestCadence {
    Instant,
    Every15Min,
    #[default]
    Hourly,
    Every4Hours,
    Daily,
}

impl DigestCadence {
    /// Wire form used by HTML form submissions. Stable: this string also
    /// appears in serde JSON since the enum uses `rename_all = "snake_case"`.
    pub fn as_str(self) -> &'static str {
        match self {
            DigestCadence::Instant => "instant",
            DigestCadence::Every15Min => "every15_min",
            DigestCadence::Hourly => "hourly",
            DigestCadence::Every4Hours => "every4_hours",
            DigestCadence::Daily => "daily",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "instant" => DigestCadence::Instant,
            "every15_min" | "every_15_min" => DigestCadence::Every15Min,
            "every4_hours" | "every_4_hours" => DigestCadence::Every4Hours,
            "daily" => DigestCadence::Daily,
            _ => DigestCadence::Hourly,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DigestCadence::Instant => "Instant",
            DigestCadence::Every15Min => "Every 15 min",
            DigestCadence::Hourly => "Hourly",
            DigestCadence::Every4Hours => "Every 4 hours",
            DigestCadence::Daily => "Daily",
        }
    }

    /// True if the cron tick at this hour:minute should consider this
    /// tenant due. The 15-minute cron is the minimum granularity, so
    /// `Every15Min` always fires; coarser cadences fire only when the tick
    /// aligns with their boundary (e.g. `Hourly` at minute :00, `Daily` at
    /// 06:00 UTC).
    pub fn is_due_at(self, hour: u32, minute: u32) -> bool {
        match self {
            DigestCadence::Instant => true,
            DigestCadence::Every15Min => true,
            DigestCadence::Hourly => minute < 15,
            DigestCadence::Every4Hours => minute < 15 && hour % 4 == 0,
            DigestCadence::Daily => minute < 15 && hour == 6,
        }
    }
}

#[cfg(test)]
mod enum_tests {
    use super::{ApprovalDecider, GrantCadence, OnboardingStep, Plan};

    #[test]
    fn grant_cadence_wire_round_trip() {
        let cases = [
            GrantCadence::Daily,
            GrantCadence::MonthlyFirst,
            GrantCadence::Weekly(0), // sun
            GrantCadence::Weekly(1), // mon
            GrantCadence::Weekly(6), // sat
        ];
        for c in cases {
            let wire = c.as_wire();
            let parsed = GrantCadence::from_wire(&wire).expect("round trip");
            assert_eq!(parsed, c);
        }
    }

    #[test]
    fn grant_cadence_unknown_wire_returns_none() {
        assert!(GrantCadence::from_wire("monthly").is_none());
        assert!(GrantCadence::from_wire("weekly_xyz").is_none());
        assert!(GrantCadence::from_wire("").is_none());
    }

    #[test]
    fn approval_decider_round_trip() {
        let cases = [
            ApprovalDecider::Discord {
                user_id: "12345".into(),
            },
            ApprovalDecider::Web {
                email: "owner@example.com".into(),
            },
            ApprovalDecider::Expired,
        ];
        for d in cases {
            let wire = d.wire();
            let parsed = ApprovalDecider::from_wire(&wire).expect("round trip");
            assert_eq!(parsed, d);
        }
    }

    #[test]
    fn approval_decider_from_unknown_returns_none() {
        assert_eq!(ApprovalDecider::from_wire("bogus:value"), None);
        assert_eq!(ApprovalDecider::from_wire(""), None);
    }

    #[test]
    fn onboarding_step_round_trip_and_index() {
        for step in [
            OnboardingStep::Basics,
            OnboardingStep::Channels,
            OnboardingStep::Notifications,
            OnboardingStep::Replies,
            OnboardingStep::Launch,
        ] {
            assert_eq!(OnboardingStep::from_wire(step.as_str()), Some(step));
        }
        assert_eq!(OnboardingStep::from_wire("welcome"), None);
        // Indices stay in display order.
        assert!(OnboardingStep::Basics.index() < OnboardingStep::Launch.index());
    }

    #[test]
    fn plan_from_wire_rejects_unknown() {
        assert_eq!(Plan::from_wire("free"), Some(Plan::Free));
        assert_eq!(Plan::from_wire("enterprise"), None);
    }
}

#[cfg(test)]
mod cadence_tests {
    use super::DigestCadence;

    #[test]
    fn instant_always_due() {
        for h in 0..24 {
            for m in [0_u32, 15, 30, 45] {
                assert!(DigestCadence::Instant.is_due_at(h, m));
            }
        }
    }

    #[test]
    fn every_15_min_always_due() {
        for h in 0..24 {
            for m in [0_u32, 15, 30, 45] {
                assert!(DigestCadence::Every15Min.is_due_at(h, m));
            }
        }
    }

    #[test]
    fn hourly_fires_only_in_first_quarter() {
        assert!(DigestCadence::Hourly.is_due_at(10, 0));
        assert!(DigestCadence::Hourly.is_due_at(10, 14));
        assert!(!DigestCadence::Hourly.is_due_at(10, 15));
        assert!(!DigestCadence::Hourly.is_due_at(10, 45));
    }

    #[test]
    fn every4_fires_only_at_aligned_hours() {
        assert!(DigestCadence::Every4Hours.is_due_at(0, 0));
        assert!(DigestCadence::Every4Hours.is_due_at(4, 14));
        assert!(DigestCadence::Every4Hours.is_due_at(8, 0));
        assert!(!DigestCadence::Every4Hours.is_due_at(2, 0));
        assert!(!DigestCadence::Every4Hours.is_due_at(4, 30));
    }

    #[test]
    fn daily_fires_only_at_6_utc_first_quarter() {
        assert!(DigestCadence::Daily.is_due_at(6, 0));
        assert!(DigestCadence::Daily.is_due_at(6, 14));
        assert!(!DigestCadence::Daily.is_due_at(6, 15));
        assert!(!DigestCadence::Daily.is_due_at(7, 0));
    }
}

/// Steps in the onboarding wizard, in display order. The wizard URL
/// (`/admin/wizard/<step>`) mirrors `as_str` exactly.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingStep {
    #[default]
    Basics,
    Channels,
    Notifications,
    Replies,
    Launch,
}

impl OnboardingStep {
    pub fn as_str(self) -> &'static str {
        match self {
            OnboardingStep::Basics => "basics",
            OnboardingStep::Channels => "channels",
            OnboardingStep::Notifications => "notifications",
            OnboardingStep::Replies => "replies",
            OnboardingStep::Launch => "launch",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "basics" => Some(OnboardingStep::Basics),
            "channels" => Some(OnboardingStep::Channels),
            "notifications" => Some(OnboardingStep::Notifications),
            "replies" => Some(OnboardingStep::Replies),
            "launch" => Some(OnboardingStep::Launch),
            _ => None,
        }
    }

    /// Display index used by the progress bar. Same order as the variant
    /// definition so adding a step doesn't drift this from `as_str`.
    pub fn index(self) -> usize {
        match self {
            OnboardingStep::Basics => 0,
            OnboardingStep::Channels => 1,
            OnboardingStep::Notifications => 2,
            OnboardingStep::Replies => 3,
            OnboardingStep::Launch => 4,
        }
    }
}

/// Onboarding state for the setup wizard.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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
    pub persona: PersonaConfig,
    /// Default wait_seconds copied into ReplyConfig on every channel account
    /// this tenant connects later. Per-account overrides live on each ReplyConfig.
    #[serde(default = "default_wait_seconds")]
    pub default_wait_seconds: u32,
    pub completed: bool,
    /// Tenant has dismissed the one-time banner explaining that AI replies
    /// now pause for review when a draft mentions money or makes a commitment.
    /// Sticky across sessions.
    #[serde(default)]
    pub risk_gate_banner_dismissed: bool,
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
            source: PersonaSource::Builder(PersonaBuilder::default()),
            safety: PersonaSafety::default(),
        }
    }
}

impl PersonaConfig {
    /// The actual prompt sent to the LLM. Computed from the source on demand.
    pub fn active_prompt(&self) -> String {
        self.source.active_prompt()
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

/// How a persona's editable middle is sourced. Tenants pick exactly one.
/// `Preset` was a third variant in older revisions; tenants picking a
/// catalog persona at onboarding now copy that row's `Builder` snapshot
/// here directly, so a tenant's KV record is always one of these two
/// pure-function shapes.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PersonaSource {
    /// User-filled inputs the system composes through `personas::generate`.
    Builder(PersonaBuilder),
    /// Power-user override: raw prompt text. Used by the homepage Concierge
    /// demo (whose middle is bespoke product copy) and by tenants who want
    /// to bypass the formula. Still envelope-wrapped at AI-call time.
    Custom(String),
}

impl PersonaSource {
    /// Pure: render the editable middle. The envelope (`crate::prompt::wrap`)
    /// is added separately at the AI-call boundary.
    pub fn active_prompt(&self) -> String {
        match self {
            PersonaSource::Builder(b) => crate::personas::generate(b),
            PersonaSource::Custom(s) => s.clone(),
        }
    }
}

/// Voice archetype — the *tone* a persona speaks in. Decoupled from
/// business type: a "Friendly" archetype works for a florist, a clinic,
/// or a tech-support helpdesk. Catalog rows in D1 carry one of these.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PersonaPreset {
    Friendly,
    Professional,
    Playful,
    Formal,
}

impl Default for PersonaPreset {
    fn default() -> Self {
        Self::Friendly
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PersonaBuilder {
    /// The voice archetype (Friendly / Professional / Playful / Formal).
    #[serde(default)]
    pub archetype: PersonaPreset,
    /// Required: the business's name (used in the generated prompt as
    /// "Business: {biz_name}, a {biz_type}…").
    #[serde(default)]
    pub biz_name: String,
    #[serde(default)]
    pub biz_type: String,
    #[serde(default)]
    pub city: String,
    /// Free-text business hours (e.g. "Tue–Sun 9am–7pm"). Optional.
    /// Plugged into the generated prompt as a "Hours: …" line so the
    /// AI can reference them when a customer asks "are you open?".
    #[serde(default)]
    pub hours: String,
    /// The single outcome the AI should drive customers toward
    /// (e.g. "book a delivery slot"). Optional but strongly encouraged.
    /// When blank, [`crate::personas::generate`] emits a default
    /// "answer the question and let them know a human will follow up"
    /// goal so every prompt has a concrete endpoint.
    #[serde(default)]
    pub goal: String,
    /// Optional landing place for the goal: a relative path (`/book`) or
    /// an absolute `https://` URL. Sanitised on save —
    /// `javascript:`/`data:`/bare domains are rejected.
    #[serde(default)]
    pub goal_url: String,
    #[serde(default)]
    pub catch_phrases: Vec<String>,
    #[serde(default)]
    pub off_topics: Vec<String>,
    #[serde(default)]
    pub never: String,
    /// Free-text conditions under which the AI should stop trying and
    /// hand off to a human (e.g. "the customer is upset", "any refund
    /// or complaint"). Optional but recommended. Rendered as a
    /// "Hand off to a human if any of these come up: …" block. The
    /// universal triggers (model confused, customer asks for a person,
    /// medical/legal/financial/safety territory) live in the immutable
    /// postamble — these are tenant-specific additions.
    #[serde(default)]
    pub handoff_conditions: Vec<String>,
}

/// One row in the `personas` D1 catalog. Curated by management, listed
/// in the demo persona picker, copied (snapshotted) into a tenant's
/// KV blob at preset-pick time.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PersonaCatalogRow {
    pub slug: String,
    pub label: String,
    pub description: String,
    pub source: PersonaSource,
    pub greeting: String,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default)]
    pub safety: PersonaSafety,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl PersonaCatalogRow {
    /// True iff the catalog row's safety verdict is Approved. Catalog
    /// rows differ from tenant `PersonaConfig` here: every approval
    /// path for a catalog row goes through the classifier (the upsert
    /// handler always resets to Draft and enqueues a `SafetyJob`), so
    /// hash-drift detection isn't needed — Approved is Approved.
    pub fn is_safe_to_use(&self) -> bool {
        matches!(self.safety.status, PersonaSafetyStatus::Approved)
    }
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
    /// Channel where AI drafts are posted with Approve/Reject buttons.
    /// When unset, the approval queue lives only on the web page.
    #[serde(default)]
    pub approval_channel_id: Option<String>,
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
            discord_channel_id: "ch-1".into(),
            origin_channel: Channel::Email,
            origin_sender: "alice@example.com".into(),
            origin_recipient: "support@proxy.com".into(),
            tenant_id: "tenant-1".into(),
            channel_account_id: "example.com".into(),
            reply_metadata: serde_json::json!({"domain": "example.com"}),
            ai_draft: Some("Draft reply text".into()),
            created_at: "2026-01-01T00:00:00Z".into(),
            handoff_signaled_at: None,
            handoff_notified: false,
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: ConversationContext = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.origin_channel, Channel::Email);
        assert_eq!(parsed.ai_draft.as_deref(), Some("Draft reply text"));
        assert!(parsed.handoff_signaled_at.is_none());
        assert!(!parsed.handoff_notified);
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
    fn conversation_config_resolve_partial_overrides_fall_back_per_field() {
        // Only idle_gap is overridden; the other two should pick up
        // their defaults independently.
        let cfg = ConversationConfig {
            idle_gap_mins: Some(45),
            handoff_cooldown_mins: None,
            max_history_messages: None,
        };
        let win = cfg.resolve();
        assert_eq!(win.idle_gap_mins, 45);
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
        assert_eq!(back.messages[1].content, "hello!");
    }

    #[test]
    fn session_deserializes_legacy_records_without_messages_field() {
        // Old KV records (pre-conversation-history) lack messages,
        // conversation_id. With #[serde(default)] they should still
        // load — empty list, empty id — and the pipeline will mint a
        // fresh conversation_id on next turn.
        let legacy_json = r#"{
            "last_inbound_at": "2026-05-04T12:00:00.000Z",
            "handoff": null
        }"#;
        let parsed: Session = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(parsed.last_inbound_at, "2026-05-04T12:00:00.000Z");
        assert!(parsed.handoff.is_none());
        assert!(parsed.conversation_id.is_empty());
        assert!(parsed.messages.is_empty());
    }

    #[test]
    fn conversation_role_wire_strings_are_stable() {
        // Workers AI's chat schema requires "user" / "assistant" exact
        // strings; the pipeline relies on these wire values.
        assert_eq!(ConversationRole::User.as_wire(), "user");
        assert_eq!(ConversationRole::Assistant.as_wire(), "assistant");
    }
}

// ============================================================================
// Billing Types: Reply Credits
// ============================================================================

/// Source of a credit entry: determines expiry behavior.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CreditSource {
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

// ============================================================================
// Scheduled grants
// ============================================================================

/// Cadence at which a scheduled grant fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantCadence {
    Daily,
    /// Day-of-week, 0 = Sunday, 6 = Saturday.
    Weekly(u8),
    /// Fires on the 1st of every month at 00:00 UTC.
    MonthlyFirst,
}

impl GrantCadence {
    pub fn as_wire(self) -> String {
        match self {
            GrantCadence::Daily => "daily".to_string(),
            GrantCadence::Weekly(d) => {
                let codes = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];
                format!("weekly_{}", codes.get(d as usize).copied().unwrap_or("mon"))
            }
            GrantCadence::MonthlyFirst => "monthly_first".to_string(),
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "daily" => Some(GrantCadence::Daily),
            "monthly_first" => Some(GrantCadence::MonthlyFirst),
            other => other.strip_prefix("weekly_").and_then(|d| match d {
                "sun" => Some(GrantCadence::Weekly(0)),
                "mon" => Some(GrantCadence::Weekly(1)),
                "tue" => Some(GrantCadence::Weekly(2)),
                "wed" => Some(GrantCadence::Weekly(3)),
                "thu" => Some(GrantCadence::Weekly(4)),
                "fri" => Some(GrantCadence::Weekly(5)),
                "sat" => Some(GrantCadence::Weekly(6)),
                _ => None,
            }),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GrantCadence::Daily => "Every day at 00:00 UTC",
            GrantCadence::Weekly(0) => "Every Sunday",
            GrantCadence::Weekly(1) => "Every Monday",
            GrantCadence::Weekly(2) => "Every Tuesday",
            GrantCadence::Weekly(3) => "Every Wednesday",
            GrantCadence::Weekly(4) => "Every Thursday",
            GrantCadence::Weekly(5) => "Every Friday",
            GrantCadence::Weekly(6) => "Every Saturday",
            GrantCadence::Weekly(_) => "Every week",
            GrantCadence::MonthlyFirst => "1st of every month",
        }
    }
}

/// Recurring credit grant. Always applies to every tenant — there is no
/// per-grant audience selector, on the assumption that operator-controlled
/// recurring drops are always platform-wide.
#[derive(Debug, Clone)]
pub struct ScheduledGrant {
    pub id: String,
    pub cadence: GrantCadence,
    pub credits: i64,
    pub expires_in_days: i64,
    pub last_run_at: Option<String>,
    pub next_run_at: String,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}
