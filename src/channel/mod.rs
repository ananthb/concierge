pub mod whatsapp;

use worker::*;

use crate::types::{Channel, InboundMessage};

/// Dispatch an outbound reply to the matching channel adapter.
pub async fn send_reply(
    env: &Env,
    msg: &InboundMessage,
    reply: &str,
    account_token: Option<&str>,
) -> Result<()> {
    let target = msg.conversation_key();
    match msg.channel {
        Channel::WhatsApp => {
            whatsapp::send_reply(env, &msg.raw_metadata, &target, reply, account_token).await
        }
    }
}

/// Dispatch a typing indicator / read receipt to the matching channel adapter.
pub async fn send_typing_indicator(
    env: &Env,
    channel: &Channel,
    metadata: &serde_json::Value,
    account_token: Option<&str>,
) -> Result<()> {
    match channel {
        Channel::WhatsApp => whatsapp::send_typing_indicator(env, metadata, account_token).await,
    }
}
