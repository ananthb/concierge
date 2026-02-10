use serde::{Deserialize, Serialize};

// ============================================================================
// Form Types (from form-worker)
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    Email,
    Mobile,
    Phone,
    LongText,
    File,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ResponderChannel {
    TwilioSms,
    TwilioRcs,
    TwilioWhatsapp,
    TwilioEmail,
    ResendEmail,
    MetaWhatsapp,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum DigestFrequency {
    #[default]
    None,
    Daily,
    Weekly,
}


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DigestConfig {
    pub frequency: DigestFrequency,
    #[serde(default)]
    pub responders: Vec<FormResponder>,
    #[serde(default)]
    pub last_sent_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FormField {
    pub id: String,
    pub label: String,
    pub field_type: FieldType,
    pub required: bool,
    pub placeholder: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FormStyle {
    pub font_family: String,
    pub font_size: String,
    pub text_color: String,
    pub text_muted: String,
    pub bg_color: String,
    pub form_bg: String,
    pub border_color: String,
    pub border_radius: String,
    pub primary_color: String,
    pub primary_hover: String,
    pub input_padding: String,
    pub success_bg: String,
    pub success_color: String,
    pub success_border: String,
    pub error_bg: String,
    pub error_color: String,
    pub error_border: String,
    #[serde(default)]
    pub custom_css: String,
    #[serde(default = "default_true")]
    pub show_title: bool,
}

fn default_true() -> bool {
    true
}

impl Default for FormStyle {
    fn default() -> Self {
        Self {
            font_family: "inherit".into(),
            font_size: "1rem".into(),
            text_color: "#333333".into(),
            text_muted: "#555555".into(),
            bg_color: "transparent".into(),
            form_bg: "#ffffff".into(),
            border_color: "#dddddd".into(),
            border_radius: "4px".into(),
            primary_color: "#0070f3".into(),
            primary_hover: "#0060df".into(),
            input_padding: "0.75rem".into(),
            success_bg: "#d4edda".into(),
            success_color: "#155724".into(),
            success_border: "#c3e6cb".into(),
            error_bg: "#f8d7da".into(),
            error_color: "#721c24".into(),
            error_border: "#f5c6cb".into(),
            custom_css: String::new(),
            show_title: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FormResponder {
    pub id: String,
    pub name: String,
    pub channel: ResponderChannel,
    pub target_field: String,
    pub subject: String,
    pub body: String,
    pub enabled: bool,
    #[serde(default)]
    pub use_ai: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FormConfig {
    pub slug: String,
    pub name: String,
    pub title: String,
    pub submit_button_text: String,
    pub success_message: String,
    pub allowed_origins: Vec<String>,
    pub fields: Vec<FormField>,
    pub style: FormStyle,
    #[serde(default)]
    pub responders: Vec<FormResponder>,
    #[serde(default)]
    pub digest: DigestConfig,
    #[serde(default)]
    pub google_sheet_url: Option<String>,
    #[serde(default)]
    pub instagram_sources: Vec<InstagramSource>,
    #[serde(default)]
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl FormConfig {
    pub fn default_fields() -> Vec<FormField> {
        vec![
            FormField {
                id: "name".into(),
                label: "Name".into(),
                field_type: FieldType::Text,
                required: true,
                placeholder: Some("Your name".into()),
            },
            FormField {
                id: "email".into(),
                label: "Email".into(),
                field_type: FieldType::Email,
                required: true,
                placeholder: Some("your@email.com".into()),
            },
            FormField {
                id: "message".into(),
                label: "Message".into(),
                field_type: FieldType::LongText,
                required: true,
                placeholder: Some("Your message...".into()),
            },
        ]
    }
}

#[derive(Debug)]
pub struct Submission {
    pub id: i64,
    pub fields_data: serde_json::Map<String, serde_json::Value>,
    pub created_at: String,
}

// ============================================================================
// Calendar Types (from calendar-worker)
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CalendarConfig {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub timezone: String,
    pub booking_links: Vec<BookingLink>,
    pub view_links: Vec<ViewLink>,
    pub feed_links: Vec<FeedLink>,
    #[serde(default)]
    pub instagram_sources: Vec<InstagramSource>,
    pub style: CalendarStyle,
    pub allowed_origins: Vec<String>,
    #[serde(default)]
    pub digest: DigestConfig,
    #[serde(default)]
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::from("New Calendar"),
            description: None,
            timezone: String::from("UTC"),
            booking_links: Vec::new(),
            view_links: Vec::new(),
            feed_links: Vec::new(),
            instagram_sources: Vec::new(),
            style: CalendarStyle::default(),
            allowed_origins: Vec::new(),
            digest: DigestConfig::default(),
            archived: false,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BookingLink {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub duration: i32,
    pub buffer_before: i32,
    pub buffer_after: i32,
    pub min_notice: i32,
    pub max_advance: i32,
    pub fields: Vec<BookingField>,
    pub confirmation_message: String,
    pub enabled: bool,
    #[serde(default)]
    pub responders: Vec<FormResponder>,
    #[serde(default = "default_true")]
    pub auto_accept: bool,
    #[serde(default)]
    pub admin_responders: Vec<FormResponder>,
    #[serde(default)]
    pub hide_title: bool,
}

impl Default for BookingLink {
    fn default() -> Self {
        Self {
            id: String::new(),
            slug: String::new(),
            name: String::from("Book a Meeting"),
            description: None,
            duration: 30,
            buffer_before: 0,
            buffer_after: 0,
            min_notice: 24,
            max_advance: 30,
            fields: vec![
                BookingField {
                    id: String::from("name"),
                    label: String::from("Name"),
                    field_type: FieldType::Text,
                    required: true,
                    placeholder: Some(String::from("Your name")),
                },
                BookingField {
                    id: String::from("email"),
                    label: String::from("Email"),
                    field_type: FieldType::Email,
                    required: true,
                    placeholder: Some(String::from("your@email.com")),
                },
            ],
            confirmation_message: String::from("Your booking has been confirmed!"),
            enabled: true,
            responders: Vec::new(),
            auto_accept: true,
            admin_responders: Vec::new(),
            hide_title: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BookingField {
    pub id: String,
    pub label: String,
    pub field_type: FieldType,
    pub required: bool,
    pub placeholder: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ViewLink {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub view_type: ViewType,
    pub date_range: Option<DateRange>,
    #[serde(default = "default_true")]
    pub show_events: bool,
    #[serde(default = "default_true")]
    pub show_event_details: bool,
    #[serde(default = "default_true")]
    pub show_bookings: bool,
    #[serde(default = "default_true")]
    pub show_booking_details: bool,
    pub enabled: bool,
}

impl Default for ViewLink {
    fn default() -> Self {
        Self {
            id: String::new(),
            slug: String::new(),
            name: String::from("Calendar View"),
            view_type: ViewType::Month,
            date_range: None,
            show_events: true,
            show_event_details: true,
            show_bookings: true,
            show_booking_details: true,
            enabled: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ViewType {
    Week,
    Month,
    Year,
    Endless,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DateRange {
    pub start: String,
    pub end: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FeedLink {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub token: String,
    pub include_details: bool,
    pub enabled: bool,
}

impl Default for FeedLink {
    fn default() -> Self {
        Self {
            id: String::new(),
            slug: String::new(),
            name: String::from("Calendar Feed"),
            token: String::new(),
            include_details: true,
            enabled: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CalendarStyle {
    pub primary_color: String,
    pub text_color: String,
    pub background_color: String,
    pub border_radius: String,
    pub font_family: String,
    #[serde(default)]
    pub custom_css: String,
}

impl Default for CalendarStyle {
    fn default() -> Self {
        Self {
            primary_color: String::from("#0070f3"),
            text_color: String::from("#333333"),
            background_color: String::from("#ffffff"),
            border_radius: String::from("4px"),
            font_family: String::from("system-ui, -apple-system, sans-serif"),
            custom_css: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CalendarEvent {
    pub id: String,
    pub calendar_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub all_day: bool,
    pub recurrence_rule: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimeSlot {
    pub id: String,
    pub calendar_id: String,
    pub day_of_week: Option<i32>,
    pub specific_date: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub slot_duration: i32,
    pub buffer_time: i32,
    pub max_bookings: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Booking {
    pub id: String,
    pub calendar_id: String,
    pub booking_link_id: String,
    pub slot_date: String,
    pub slot_time: String,
    pub duration: i32,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub notes: Option<String>,
    pub fields_data: Option<serde_json::Value>,
    pub status: BookingStatus,
    pub confirmation_token: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BookingStatus {
    Pending,
    Confirmed,
    Cancelled,
    Completed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AvailableSlot {
    pub date: String,
    pub time: String,
    pub end_time: String,
    pub available: bool,
}

// ============================================================================
// Instagram Integration Types
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstagramSource {
    pub id: String,
    pub instagram_user_id: String,
    pub instagram_username: String,
    pub enabled: bool,
    pub last_synced_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingStatus {
    Pending,
    Processed,
    NoEvent,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessedPost {
    pub id: String,
    pub calendar_id: Option<String>,
    pub form_slug: Option<String>,
    pub instagram_source_id: String,
    pub instagram_post_id: String,
    pub instagram_permalink: String,
    pub caption_hash: String,
    pub event_id: Option<String>,
    pub contact_id: Option<i64>,
    pub event_signature: Option<String>,
    pub processing_status: ProcessingStatus,
    pub ai_response: Option<String>,
    pub processed_at: String,
    pub updated_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExtractedEvent {
    pub title: Option<String>,
    pub date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub description: Option<String>,
    pub is_cancellation: bool,
    pub confidence: f32,
}

impl Default for ExtractedEvent {
    fn default() -> Self {
        Self {
            title: None,
            date: None,
            start_time: None,
            end_time: None,
            description: None,
            is_cancellation: false,
            confidence: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EventSource {
    pub id: String,
    pub event_id: Option<String>,
    pub contact_id: Option<i64>,
    pub source_type: String,
    pub source_id: String,
    pub external_id: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstagramToken {
    pub access_token: String,
    pub expires_at: String,
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstagramMedia {
    pub id: String,
    pub caption: Option<String>,
    pub media_type: String,
    pub permalink: String,
    pub timestamp: String,
}

// ============================================================================
// WhatsApp Types
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
