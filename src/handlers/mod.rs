//! Handler modules for the concierge worker

mod admin;
mod admin_billing;
mod admin_email;
mod admin_instagram;
mod admin_lead_forms;
mod admin_whatsapp;
pub mod auth;
mod data_deletion;
mod instagram_oauth;
mod instagram_webhook;
mod lead_form;
pub mod onboarding;
mod webhook;
mod whatsapp_signup;

pub use admin::handle_admin;
pub use auth::handle_auth;
pub use data_deletion::handle_data_deletion;
pub use instagram_oauth::handle_instagram;
pub use lead_form::handle_lead_form;
pub use webhook::handle_webhook;
pub use whatsapp_signup::handle_whatsapp_signup;

use worker::Request;

/// Extract base URL from request
pub(crate) fn get_base_url(req: &Request) -> String {
    match req.url() {
        Ok(url) => format!(
            "{}://{}",
            url.scheme(),
            url.host_str().unwrap_or("localhost")
        ),
        Err(_) => String::from("https://localhost"),
    }
}

/// Extract Origin header from request
pub(crate) fn get_origin(req: &Request) -> Option<String> {
    req.headers().get("Origin").ok().flatten()
}
