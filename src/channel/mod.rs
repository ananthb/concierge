pub mod whatsapp;

use worker::*;

use crate::types::Channel;

/// Dispatch a reply to the correct channel adapter.
///
/// WhatsApp is the only inbound channel at launch. The match is kept
/// (rather than calling `whatsapp::send_reply` directly) so adding a
/// channel back is a variant plus an arm, not a re-plumb of the pipeline.
pub async fn send_reply(
    channel: &Channel,
    env: &Env,
    metadata: &serde_json::Value,
    to: &str,
    body: &str,
) -> Result<()> {
    match channel {
        Channel::WhatsApp => whatsapp::send_reply(env, metadata, to, body).await,
    }
}
