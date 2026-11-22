//! Cloudflare Queue producer + consumer for tenant content safety checks.
//!
//! Enqueues a `SafetyJob` whenever a tenant updates their `brand_name`
//! (`PUT /api/wizard/basics`) or saves a canned reply (`PUT /api/whatsapp/:id`).
//! The consumer classifies the text with `@cf/meta/llama-guard-3-8b` and writes
//! the `SafetyState` verdict back to KV if the content hash has not drifted.

use serde::{Deserialize, Serialize};
use worker::MessageExt;
use worker::*;

use crate::helpers::{now_iso, sha256_hex};
use crate::safety::{classify_text, SafetyVerdict};
use crate::storage::{
    get_onboarding, get_whatsapp_account, save_onboarding, save_whatsapp_account,
};
use crate::types::{ReplyResponse, SafetyState, SafetyStatus};

pub const QUEUE_BINDING: &str = "SAFETY_QUEUE";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SafetyTarget {
    TenantProfile,
    CannedReply { account_id: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SafetyJob {
    pub tenant_id: String,
    pub target: SafetyTarget,
    pub content_hash: String,
}

/// Send a safety job onto the queue. Logs and returns `Ok(())` if the queue
/// binding is missing so saves do not fail in local environments.
pub async fn enqueue(env: &Env, job: SafetyJob) -> Result<()> {
    let queue = match env.queue(QUEUE_BINDING) {
        Ok(q) => q,
        Err(e) => {
            console_log!(
                "Safety queue '{QUEUE_BINDING}' not configured ({e:?}); skipping enqueue."
            );
            return Ok(());
        }
    };
    queue.send(&job).await
}

/// Process one batch of safety jobs. Called from `#[event(queue)]` in `lib.rs`.
pub async fn handle_batch(batch: MessageBatch<SafetyJob>, env: Env) -> Result<()> {
    for msg_result in batch.iter() {
        let msg = match msg_result {
            Ok(m) => m,
            Err(e) => {
                console_log!("Safety queue: failed to deserialize message: {e:?}");
                continue;
            }
        };

        let job = msg.body();
        let res = match &job.target {
            SafetyTarget::TenantProfile => {
                process_tenant_profile(&env, &job.tenant_id, &job.content_hash).await
            }
            SafetyTarget::CannedReply { account_id } => {
                process_canned_reply(&env, &job.tenant_id, account_id, &job.content_hash).await
            }
        };

        match res {
            Ok(()) => msg.ack(),
            Err(()) => msg.retry(),
        }
    }

    Ok(())
}

async fn process_tenant_profile(
    env: &Env,
    tenant_id: &str,
    expected_hash: &str,
) -> std::result::Result<(), ()> {
    let kv = env.kv("KV").map_err(|e| {
        console_log!("Safety queue: KV binding missing: {e:?}");
    })?;

    let mut state = get_onboarding(&kv, tenant_id).await.map_err(|e| {
        console_log!("Safety queue: load onboarding for {tenant_id} failed: {e:?}");
    })?;

    let current_hash = sha256_hex(&state.business.name);
    if current_hash != expected_hash {
        console_log!("Safety queue: stale profile job for {tenant_id}, skipping");
        return Ok(());
    }

    let verdict = classify_text(env, &state.business.name).await;
    state.profile_safety = verdict_to_state(verdict, current_hash);

    save_onboarding(&kv, tenant_id, &state).await.map_err(|e| {
        console_log!("Safety queue: write-back for tenant {tenant_id} failed: {e:?}");
    })
}

async fn process_canned_reply(
    env: &Env,
    tenant_id: &str,
    account_id: &str,
    expected_hash: &str,
) -> std::result::Result<(), ()> {
    let kv = env.kv("KV").map_err(|e| {
        console_log!("Safety queue: KV binding missing: {e:?}");
    })?;

    let Some(mut account) = get_whatsapp_account(&kv, account_id).await.map_err(|e| {
        console_log!("Safety queue: load whatsapp account {account_id} failed: {e:?}");
    })?
    else {
        return Ok(());
    };

    if account.tenant_id != tenant_id {
        return Ok(());
    }

    let ReplyResponse::Canned { ref text } = account.auto_reply.response else {
        console_log!("Safety queue: account {account_id} switched off canned mode, skipping");
        return Ok(());
    };

    let current_hash = sha256_hex(text);
    if current_hash != expected_hash {
        console_log!("Safety queue: stale canned job for {account_id}, skipping");
        return Ok(());
    }

    let verdict = classify_text(env, text).await;
    account.canned_safety = verdict_to_state(verdict, current_hash);
    account.updated_at = now_iso();

    save_whatsapp_account(&kv, &account).await.map_err(|e| {
        console_log!("Safety queue: write-back for account {account_id} failed: {e:?}");
    })
}

pub fn verdict_to_state(verdict: SafetyVerdict, content_hash: String) -> SafetyState {
    let now = now_iso();
    match verdict {
        SafetyVerdict::Approved => SafetyState {
            status: SafetyStatus::Approved,
            content_hash: Some(content_hash),
            checked_at: Some(now),
            vague_reason: None,
        },
        SafetyVerdict::Rejected { vague_reason } => SafetyState {
            status: SafetyStatus::Rejected,
            content_hash: Some(content_hash),
            checked_at: Some(now),
            vague_reason: Some(vague_reason),
        },
    }
}
