//! Email channel: inbound via Cloudflare Email Routing, outbound via
//! Cloudflare Email Service.
//!
//! `inbound` parses a `ForwardableEmailMessage` into the shared pipeline;
//! `mime` extracts the body from raw RFC 5322 bytes; `send` is the
//! structured-message outbound path.

pub mod inbound;
pub mod mime;
pub mod send;
