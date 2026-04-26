//! Cloudflare Queue producer + consumer for persona safety checks.
//!
//! When a tenant saves a persona, the admin handler enqueues a `SafetyJob`.
//! The queue consumer (wired in `lib.rs` via `#[event(queue)]`) re-reads the
//! persona, confirms the prompt hash hasn't drifted (a newer save would
//! supersede this job), runs the classifier, and writes the verdict back.
//!
//! Failures are surfaced via `message.retry()` so the queue's DLQ retry
//! policy applies.

use serde::{Deserialize, Serialize};
use worker::*;
// `ack()` and `retry()` come from the `MessageExt` trait.
use worker::MessageExt;

use crate::safety::{classify_persona, SafetyVerdict};
use crate::storage::{get_onboarding, save_onboarding};
use crate::types::{PersonaSafety, PersonaSafetyStatus};

/// Queue binding name in `wrangler.toml`.
pub const QUEUE_BINDING: &str = "SAFETY_QUEUE";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SafetyJob {
    pub tenant_id: String,
    /// SHA-256 of the active prompt at the time the job was enqueued. The
    /// consumer ignores the job if the persona's current active prompt has a
    /// different hash, on the assumption that a newer save has already
    /// re-enqueued.
    pub prompt_hash: String,
}

/// Send a safety job onto the queue. Logs and returns `Ok(())` if the queue
/// binding is missing (e.g. local dev without a queue configured) so the
/// caller's save flow always succeeds — the persona will simply remain in
/// `Pending` until the next save reaches a properly-bound environment.
pub async fn enqueue(env: &Env, job: SafetyJob) -> Result<()> {
    let queue = match env.queue(QUEUE_BINDING) {
        Ok(q) => q,
        Err(e) => {
            console_log!(
                "Safety queue '{QUEUE_BINDING}' not configured ({e:?}); skipping enqueue. \
                 Persona stays Pending."
            );
            return Ok(());
        }
    };
    queue.send(&job).await
}

/// Process one batch of safety jobs. Called from the worker's `#[event(queue)]`
/// handler in `lib.rs`. Each job is acknowledged on success, retried on
/// transient KV errors so the queue's retry policy can take over.
pub async fn handle_batch(batch: MessageBatch<SafetyJob>, env: Env) -> Result<()> {
    let kv = env.kv("KV")?;

    for msg_result in batch.iter() {
        let msg = match msg_result {
            Ok(m) => m,
            Err(e) => {
                console_log!("Safety queue: failed to deserialize message: {e:?}");
                continue;
            }
        };

        let job = msg.body();

        let mut state = match get_onboarding(&kv, &job.tenant_id).await {
            Ok(s) => s,
            Err(e) => {
                console_log!(
                    "Safety queue: load onboarding for {} failed: {e:?}",
                    job.tenant_id
                );
                msg.retry();
                continue;
            }
        };

        // Stale-check: if the active prompt has changed since this job was
        // enqueued, a newer save has already re-enqueued. Drop this one.
        let current_hash = state.persona.active_prompt_hash();
        if current_hash != job.prompt_hash {
            console_log!(
                "Safety queue: stale job for {} (hash drifted), skipping",
                job.tenant_id
            );
            msg.ack();
            continue;
        }

        let verdict = classify_persona(&env, &state.persona.active_prompt()).await;
        let now = crate::helpers::now_iso();
        state.persona.safety = match verdict {
            SafetyVerdict::Approved => PersonaSafety {
                status: PersonaSafetyStatus::Approved,
                checked_prompt_hash: Some(current_hash),
                checked_at: Some(now),
                vague_reason: None,
            },
            SafetyVerdict::Rejected { vague_reason } => PersonaSafety {
                status: PersonaSafetyStatus::Rejected,
                checked_prompt_hash: Some(current_hash),
                checked_at: Some(now),
                vague_reason: Some(vague_reason),
            },
        };

        match save_onboarding(&kv, &job.tenant_id, &state).await {
            Ok(()) => msg.ack(),
            Err(e) => {
                console_log!(
                    "Safety queue: write-back for {} failed: {e:?}",
                    job.tenant_id
                );
                msg.retry();
            }
        }
    }

    Ok(())
}
