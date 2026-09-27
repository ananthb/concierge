//! Handler modules for the concierge worker

pub mod auth;
mod data_deletion;
mod demo_chat;
pub mod demo_personas_list;
pub mod health;
mod webhook;
mod whatsapp_signup;

pub use auth::handle_auth;
pub use data_deletion::handle_data_deletion;
pub use demo_chat::handle_demo_chat;
pub use webhook::handle_webhook;
pub use whatsapp_signup::handle_whatsapp_signup;

use worker::Request;

/// Extract base URL (scheme + host + port) from a request.
pub(crate) fn get_base_url(req: &Request) -> String {
    match req.url() {
        Ok(url) => match (url.host_str(), url.port()) {
            (Some(host), Some(port)) => format!("{}://{}:{}", url.scheme(), host, port),
            (Some(host), None) => format!("{}://{}", url.scheme(), host),
            (None, _) => format!("{}://localhost", url.scheme()),
        },
        Err(_) => String::from("https://localhost"),
    }
}
