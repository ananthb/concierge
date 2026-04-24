//! Template rendering for HTML pages

mod admin;
pub mod admin_email;
pub mod base;
pub mod billing;
pub mod credit_slider;
pub mod discord;
mod lead_form;
pub mod management;
pub mod onboarding;

pub use admin::*;
pub use lead_form::*;

/// Hash character constant for use in format strings (avoids escaping issues)
pub(crate) const HASH: &str = "#";
