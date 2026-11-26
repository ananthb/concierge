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
