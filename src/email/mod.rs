//! Email channel: inbound via Cloudflare Email Routing, outbound via
//! Cloudflare Email Service.
//!
//! `inbound` parses a `ForwardableEmailMessage` into the shared pipeline;
//! `mime` extracts the body from raw RFC 5322 bytes; `send` is the
//! structured-message outbound path; `onboarding` wires a tenant's zone into
//! Email Routing via the Cloudflare API.

pub mod inbound;
pub mod mime;
pub mod onboarding;
pub mod send;
