use serde::{Deserialize, Serialize};

use crate::locale::Currency;

// ============================================================================
// Tenant Types
// ============================================================================

/// Tenant subscription tier. Pricing pages (`templates/management.rs`)
/// match on this enum; rate-limit/quota logic that needs a plan branch
/// adds a method here so adding a tier doesn't fan out across files.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    /// Complimentary account: usage is tracked (replies_used increments,
    /// dashboards still show a balance) but the credit gate is skipped
    /// and Razorpay CTAs are hidden. Operators flip selected tenants to
    /// Free from the management panel.
    Free,
    /// Standard pay-per-credit account.
    #[default]
    Paid,
}

impl Plan {
    pub const ALL: &'static [Plan] = &[Plan::Free, Plan::Paid];

    pub fn as_str(self) -> &'static str {
        match self {
            Plan::Free => "free",
            Plan::Paid => "paid",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Plan::Free => "Free",
            Plan::Paid => "Paid",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "free" => Some(Plan::Free),
            "paid" => Some(Plan::Paid),
            _ => None,
        }
    }

    /// Whether the credit balance gates outbound replies. Free accounts
    /// still consume credits when present, but never get blocked when
    /// the balance hits zero.
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
    /// BCP-47 locale tag, e.g. "en-IN", "en-US". Drives UI grouping and
    /// (in Phase 2) translated copy. Currency below is a separate override
    /// that lets a tenant see prices in INR while reading English-IN copy.
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub currency: Currency,
    /// Set the first time we observe a captured Razorpay payment for this
    /// tenant. The sign-up wizard charges a small refundable verification
    /// amount; this flips on success and gates wizard "Finish".
    #[serde(default)]
    pub verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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

/// Per-channel reply behaviour: one response for every inbound message.
///
/// The ordered rule list (keyword and embedding matchers, per-rule approval
/// policy) was cut. Every inbound now takes the same path: `Canned` sends
/// verbatim, `Prompt` runs the persona through the LLM.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReplyConfig {
    pub enabled: bool,
    /// What to send. Was `default_rule`, the mandatory fallback that in
    /// practice handled nearly every message anyway.
    #[serde(default = "default_response", alias = "default_rule")]
    pub response: ReplyResponse,
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
            response: default_response(),
            wait_seconds: default_wait_seconds(),
        }
    }
}

impl ReplyConfig {
    /// Read the response text without unwrapping the enum.
    pub fn default_text(&self) -> &str {
        match &self.response {
            ReplyResponse::Canned { text } | ReplyResponse::Prompt { text } => text,
        }
    }

    /// True when the response is static text (no LLM, no credit).
    pub fn default_is_canned(&self) -> bool {
        matches!(self.response, ReplyResponse::Canned { .. })
    }

    /// Set the response from an API payload. `mode` is the wire value
    /// ("canned" / "prompt" / legacy "static" / "ai").
    pub fn set_default_response(&mut self, mode: &str, text: String) {
        self.response = match mode {
            "ai" | "prompt" => ReplyResponse::Prompt { text },
            _ => ReplyResponse::Canned { text },
        };
    }
}

/// Default response for a channel that hasn't been customized: run the
/// persona through the LLM with a generic instruction.
pub fn default_response() -> ReplyResponse {
    ReplyResponse::Prompt {
        text: "Reply to the customer's message helpfully.".to_string(),
    }
}

pub fn default_wait_seconds() -> u32 {
    5
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
// Unified Messaging Types
// ============================================================================

/// Inbound channel a message arrived on.
///
/// WhatsApp is the only one at launch; Instagram, Email and Discord were
/// cut. The enum stays (rather than being erased) because the D1 `channel`
/// column, the conversation key and the pipeline all carry it, and adding
/// a channel back should be a variant plus an arm.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    WhatsApp,
}

impl Channel {
    /// Lowercase wire form used by the D1 `channel` column and inside
    /// API/webhook payloads. Diverges from serde's snake_case form (which
    /// would emit "whats_app"); keep both intact.
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::WhatsApp => "whatsapp",
        }
    }

    /// Display label used in the UI.
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

/// What was done with a message after the pipeline routed it. Stored on
/// the `messages` row so an operator can audit which path fired.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageAction {
    /// Auto-reply sent.
    AutoReply,
    /// Draft tripped the risk gate: nothing was sent to the customer and
    /// the tenant was paged to take the conversation over.
    HandedOff,
}

impl MessageAction {
    pub fn as_str(self) -> &'static str {
        match self {
            MessageAction::AutoReply => "auto_reply",
            MessageAction::HandedOff => "handed_off",
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
    pub has_attachment: bool,
    pub tenant_id: String,
    pub channel_account_id: String,
    pub raw_metadata: serde_json::Value,
}

/// Per-customer conversation session. Stable-keyed by
/// `(tenant_id, channel, channel_account_id, sender)` (see
/// `storage::save_session`).
///
/// Concierge doesn't have a formal "thread" object. A Session is the
/// soft equivalent. A conversation is considered ended once the
/// customer has been silent for the tenant's effective idle gap (see
/// `prompt::DEFAULT_CONVERSATION_IDLE_GAP_MINS` and
/// [`ConversationConfig`]): the next inbound after that gap starts a
/// fresh conversation (any in-progress handoff state is wiped, and
/// the message history is cleared). Within the gap, all inbound from
/// the same sender belong to the same conversation regardless of
/// length.
///
/// Sessions are the only conversation state: the per-approval
/// `ConversationContext` record went with the approval queue.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Session {
    /// RFC3339 timestamp of the most recent inbound message we
    /// processed for this thread. Updated on every inbound, including
    /// during the post-cooldown silent window. As long as the
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
/// Workers AI expects in its `messages` array: `User` for inbound
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
    /// `prompt::DEFAULT_HANDOFF_COOLDOWN_MINS` and the
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
    /// `prompt::DEFAULT_CONVERSATION_IDLE_GAP_MINS`.
    #[serde(default)]
    pub idle_gap_mins: Option<u32>,
    /// How long after a handoff signal we keep replying with the
    /// holding-pattern voice before going silent. Falls back to
    /// `prompt::DEFAULT_HANDOFF_COOLDOWN_MINS`.
    #[serde(default)]
    pub handoff_cooldown_mins: Option<u32>,
    /// How many recent turns we keep as chat context for the AI.
    /// Falls back to
    /// `prompt::DEFAULT_CONVERSATION_MAX_MESSAGES`.
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

/// Business information collected at onboarding. Used for invoicing.
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

/// Where a human handoff is announced.
///
/// Email is the only channel. The Discord relay that used to carry an
/// embed with Approve/Reject buttons was cut along with the approval
/// queue, and with the queue went the digest: a handoff pages the tenant
/// the moment it happens, so there is no cadence left to configure.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NotificationConfig {
    /// Email the account owner when a conversation is handed off. On by
    /// default — a handoff nobody hears about is a dropped customer.
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

#[cfg(test)]
mod enum_tests {
    use super::{OnboardingStep, Plan};

    #[test]
    fn onboarding_step_round_trip_and_index() {
        for step in OnboardingStep::ALL {
            assert_eq!(OnboardingStep::from_wire(step.as_str()), Some(step));
        }
        assert_eq!(OnboardingStep::from_wire("welcome"), None);
        // Indices stay in display order, and ALL is in that order too.
        for (i, step) in OnboardingStep::ALL.iter().enumerate() {
            assert_eq!(step.index(), i);
        }
    }

    #[test]
    fn onboarding_step_accepts_the_old_replies_name() {
        // A wizard abandoned mid-flight has "replies" persisted as its step.
        assert_eq!(
            OnboardingStep::from_wire("replies"),
            Some(OnboardingStep::Persona)
        );
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
}

/// Steps in the onboarding wizard, in display order. The wizard URL
/// (`/wizard/<step>`) mirrors `as_str` exactly.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingStep {
    #[default]
    Basics,
    Channels,
    Persona,
    Launch,
}

impl OnboardingStep {
    /// Every step in display order. The frontend renders its progress bar
    /// from this, so the wizard's shape is described in one place.
    pub const ALL: [OnboardingStep; 4] = [
        OnboardingStep::Basics,
        OnboardingStep::Channels,
        OnboardingStep::Persona,
        OnboardingStep::Launch,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            OnboardingStep::Basics => "basics",
            OnboardingStep::Channels => "channels",
            OnboardingStep::Persona => "persona",
            OnboardingStep::Launch => "launch",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "basics" => Some(OnboardingStep::Basics),
            "channels" => Some(OnboardingStep::Channels),
            // "replies" was this step's name when it also seeded per-channel
            // reply rules. Accepted so a half-finished wizard resumes.
            "persona" | "replies" => Some(OnboardingStep::Persona),
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
            OnboardingStep::Persona => 2,
            OnboardingStep::Launch => 3,
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
    pub persona: PersonaConfig,
    /// Default wait_seconds copied into ReplyConfig on every channel account
    /// this tenant connects later. Per-account overrides live on each ReplyConfig.
    #[serde(default = "default_wait_seconds")]
    pub default_wait_seconds: u32,
    #[serde(default)]
    pub completed: bool,
    /// Tenant has dismissed the one-time banner explaining that AI replies
    /// now pause for review when a draft mentions money or makes a commitment.
    /// Sticky across sessions.
    #[serde(default)]
    pub risk_gate_banner_dismissed: bool,
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self {
            step: OnboardingStep::default(),
            business: BusinessInfo::default(),
            notifications: NotificationConfig::default(),
            conversation: ConversationConfig::default(),
            persona: PersonaConfig::default(),
            default_wait_seconds: default_wait_seconds(),
            completed: false,
            risk_gate_banner_dismissed: false,
        }
    }
}

/// Tenant-wide AI persona used as the system prompt for every AI reply.
/// The persona is one of three sources (Preset, Builder, Custom), never a
/// mix, so there is exactly one source of truth for the active prompt.
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
    /// Archetype-based personas require the voice prompt fetched from D1.
    pub fn active_prompt(&self, voice_prompt: &str) -> String {
        self.source.active_prompt(voice_prompt)
    }

    /// SHA-256 of the active prompt, used to detect when a re-run of the
    /// safety classifier is needed.
    pub fn active_prompt_hash(&self, voice_prompt: &str) -> String {
        crate::helpers::sha256_hex(&self.active_prompt(voice_prompt))
    }

    /// True if AI replies are allowed: the safety check has approved the
    /// current prompt (no hash drift since approval).
    pub fn is_safe_to_use(&self, voice_prompt: &str) -> bool {
        matches!(self.safety.status, PersonaSafetyStatus::Approved)
            && self.safety.checked_prompt_hash.as_deref()
                == Some(self.active_prompt_hash(voice_prompt).as_str())
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
    /// Power-user override: raw prompt text.
    Custom(String),
}

impl PersonaSource {
    /// Pure: render the editable middle. The envelope (`crate::prompt::wrap`)
    /// is added separately at the AI-call boundary.
    pub fn active_prompt(&self, voice_prompt: &str) -> String {
        match self {
            PersonaSource::Builder(b) => crate::personas::generate(b, voice_prompt),
            PersonaSource::Custom(s) => s.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PersonaBuilder {
    /// Reference to an entry in the `archetypes` D1 table.
    #[serde(default)]
    pub archetype_slug: String,
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
    /// When blank, `personas::generate` emits a default
    /// "answer the question and let them know a human will follow up"
    /// goal so every prompt has a concrete endpoint.
    #[serde(default)]
    pub goal: String,
    /// Optional landing place for the goal: a relative path (`/book`) or
    /// an absolute `https://` URL. Sanitised on save:
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
    /// postamble. These are tenant-specific additions.
    #[serde(default)]
    pub handoff_conditions: Vec<String>,
}

/// One row in the `archetypes` D1 catalog. Curated by management, listed
/// in the demo persona picker, and referenced by a tenant's persona
/// config.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Archetype {
    pub slug: String,
    pub label: String,
    pub description: String,
    pub voice_prompt: String,
    pub greeting: String,
    pub catch_phrases: Vec<String>,
    pub off_topics: Vec<String>,
    pub never: String,
    pub handoff_conditions: Vec<String>,
    #[serde(default)]
    pub safety: PersonaSafety,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl Archetype {
    /// True iff the archetype's safety verdict is Approved.
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
    /// User-facing decline reason for the Rejected case. Always vague: the
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
        // load (empty list, empty id), and the pipeline will mint a
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
