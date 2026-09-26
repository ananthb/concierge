//! Outbound email only.
//!
//! Inbound email (Cloudflare Email Routing → `forward`/`handler`/`mime`)
//! was cut with the email channel. What remains is the send path used to
//! page a tenant when a conversation is handed off to a human.

pub mod send;
