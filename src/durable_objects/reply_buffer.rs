//! Buffers inbound messages from a single conversation for `wait_seconds`,
//! then sends one combined AI reply. Drops buffered bodies immediately
//! after the AI call returns.
//!
//! Keyed by `(tenant_id, channel, sender)`: one DO instance per
//! conversation. Sliding window: each new message resets the alarm.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use worker::*;

use crate::pipeline;
use crate::types::*;

#[derive(Serialize, Deserialize, Clone)]
struct BufferedMsg {
    id: String,
    body: String,
    sender_name: Option<String>,
    raw_metadata: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
struct ConversationCtx {
    tenant_id: String,
    channel: Channel,
    sender: String,
    recipient: String,
    channel_account_id: String,
    subject: Option<String>,
}

#[derive(Deserialize)]
struct PushPayload {
    msg: InboundMessage,
    wait_seconds: u32,
}

#[durable_object]
pub struct ReplyBufferDO {
    state: State,
    env: Env,
}

impl DurableObject for ReplyBufferDO {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        let payload: PushPayload = req.json().await?;
        let msg = payload.msg;

        let ctx = ConversationCtx {
            tenant_id: msg.tenant_id.clone(),
            channel: msg.channel.clone(),
            sender: msg.sender.clone(),
            recipient: msg.recipient.clone(),
            channel_account_id: msg.channel_account_id.clone(),
            subject: msg.subject.clone(),
        };
        self.state.storage().put("ctx", &ctx).await?;

        let mut pending: Vec<BufferedMsg> = self
            .state
            .storage()
            .get("pending")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        pending.push(BufferedMsg {
            id: msg.id,
            body: msg.body,
            sender_name: msg.sender_name,
            raw_metadata: msg.raw_metadata,
        });
        self.state.storage().put("pending", &pending).await?;

        // (Re)schedule alarm. Sliding window.
        self.state
            .storage()
            .set_alarm(Duration::from_secs(payload.wait_seconds.max(1) as u64))
            .await?;

        Response::ok("queued")
    }

    async fn alarm(&self) -> Result<Response> {
        let pending: Vec<BufferedMsg> = self
            .state
            .storage()
            .get("pending")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let ctx: Option<ConversationCtx> = self.state.storage().get("ctx").await.ok().flatten();

        // Drop everything before the LLM call. If the call fails or hangs,
        // we don't want to keep bodies around in DO storage.
        let _ = self.state.storage().delete_all().await;

        let ctx = match ctx {
            Some(c) => c,
            None => return Response::ok("no ctx"),
        };
        if pending.is_empty() {
            return Response::ok("empty");
        }

        let last = pending.last().unwrap().clone();
        let first = pending.first().unwrap().clone();
        let combined_body = pending
            .iter()
            .map(|m| m.body.clone())
            .collect::<Vec<_>>()
            .join("\n");

        let synth = InboundMessage {
            id: last.id,
            channel: ctx.channel,
            sender: ctx.sender,
            sender_name: first.sender_name,
            recipient: ctx.recipient,
            body: combined_body,
            subject: ctx.subject,
            has_attachment: false,
            tenant_id: ctx.tenant_id,
            channel_account_id: ctx.channel_account_id,
            raw_metadata: last.raw_metadata,
        };

        if let Err(e) = pipeline::process_inbound_immediate(&synth, &self.env).await {
            console_log!("ReplyBufferDO alarm error: {:?}", e);
        }

        Response::ok("done")
    }
}
