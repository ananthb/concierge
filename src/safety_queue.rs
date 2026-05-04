//! Cloudflare Queue producer + consumer for persona safety checks.
//!
//! When a tenant saves a persona OR a management user saves a catalog
//! row, the corresponding handler enqueues a `SafetyJob`. The consumer
//! (wired in `lib.rs` via `#[event(queue)]`) re-reads the persona,
//! confirms it hasn't drifted (a newer save would supersede this job),
//! runs the classifier, and writes the verdict back to either KV
//! (tenant) or D1 (catalog) depending on the job's target.
//!
//! Failures are surfaced via `message.retry()` so the queue's DLQ
//! retry policy applies.

use serde::{Deserialize, Serialize};
use worker::*;
// `ack()` and `retry()` come from the `MessageExt` trait.
use worker::MessageExt;

use crate::safety::{classify_persona, SafetyVerdict};
use crate::storage::{get_onboarding, save_onboarding};
use crate::types::{PersonaSafety, PersonaSafetyStatus};

/// Queue binding name in `wrangler.toml`.
pub const QUEUE_BINDING: &str = "SAFETY_QUEUE";

/// What this safety job is gating. Tenant jobs write the verdict back
/// to the tenant's `OnboardingState` in KV; catalog jobs write it to
/// the persona row in D1.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SafetyJobTarget {
    Tenant { tenant_id: String },
    Catalog { slug: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SafetyJob {
    /// Whether this job vets a tenant's persona or a catalog row.
    pub target: SafetyJobTarget,
    /// SHA-256 of the active prompt at the time the job was enqueued.
    /// For tenant jobs we drop the job if the prompt has drifted (a
    /// newer save will have re-enqueued). Catalog jobs don't carry a
    /// drift check. Every catalog save resets the row to Draft and
    /// enqueues a fresh job, so the latest job is always authoritative.
    pub prompt_hash: String,
}

/// Send a safety job onto the queue. Logs and returns `Ok(())` if the queue
/// binding is missing (e.g. local dev without a queue configured) so the
/// caller's save flow always succeeds. The persona stays Pending/Draft
/// until the next save reaches a properly-bound environment.
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

/// Process one batch of safety jobs. Called from the worker's
/// `#[event(queue)]` handler in `lib.rs`.
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

        match &job.target {
            SafetyJobTarget::Tenant { tenant_id } => {
                if let Err(()) = process_tenant_job(&env, tenant_id, &job.prompt_hash).await {
                    msg.retry();
                    continue;
                }
                msg.ack();
            }
            SafetyJobTarget::Catalog { slug } => {
                if let Err(()) = process_catalog_job(&env, slug).await {
                    msg.retry();
                    continue;
                }
                msg.ack();
            }
        }
    }

    Ok(())
}

async fn process_tenant_job(
    env: &Env,
    tenant_id: &str,
    expected_hash: &str,
) -> std::result::Result<(), ()> {
    let kv = env.kv("KV").map_err(|e| {
        console_log!("Safety queue: KV binding missing: {e:?}");
    })?;
    let db = env.d1("DB").map_err(|e| {
        console_log!("Safety queue: D1 binding missing: {e:?}");
    })?;

    let mut state = match get_onboarding(&kv, tenant_id).await {
        Ok(s) => s,
        Err(e) => {
            console_log!("Safety queue: load onboarding for {tenant_id} failed: {e:?}");
            return Err(());
        }
    };

    let voice_prompt = match &state.persona.source {
        crate::types::PersonaSource::Builder(b) => {
            match crate::storage::get_archetype(&db, &b.archetype_slug).await {
                Ok(Some(a)) => a.voice_prompt,
                _ => {
                    console_log!(
                        "Safety queue: archetype {} not found for tenant {tenant_id}",
                        b.archetype_slug
                    );
                    String::new()
                }
            }
        }
        crate::types::PersonaSource::Custom(_) => String::new(),
    };

    // Stale-check: if the active prompt has changed since enqueue, a
    // newer save has already re-enqueued. Drop this one.
    let current_hash = state.persona.active_prompt_hash(&voice_prompt);
    if current_hash != expected_hash {
        console_log!("Safety queue: stale tenant job for {tenant_id} (hash drifted), skipping");
        return Ok(());
    }

    let verdict = classify_persona(env, &state.persona.active_prompt(&voice_prompt)).await;
    let now = crate::helpers::now_iso();
    state.persona.safety = match verdict {
        SafetyVerdict::Approved => PersonaSafety {
            status: PersonaSafetyStatus::Approved,
            checked_prompt_hash: Some(current_hash),
            checked_at: Some(now),
            vague_reason: None,
            ..Default::default()
        },
        SafetyVerdict::Rejected { vague_reason } => PersonaSafety {
            status: PersonaSafetyStatus::Rejected,
            checked_prompt_hash: Some(current_hash),
            checked_at: Some(now),
            vague_reason: Some(vague_reason),
            ..Default::default()
        },
    };

    match save_onboarding(&kv, tenant_id, &state).await {
        Ok(()) => Ok(()),
        Err(e) => {
            console_log!("Safety queue: write-back for tenant {tenant_id} failed: {e:?}");
            Err(())
        }
    }
}

async fn process_catalog_job(env: &Env, slug: &str) -> std::result::Result<(), ()> {
    let db = env.d1("DB").map_err(|e| {
        console_log!("Safety queue: D1 binding missing: {e:?}");
    })?;

    let row = match crate::storage::get_archetype(&db, slug).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            console_log!("Safety queue: archetype {slug} no longer exists, skipping");
            return Ok(());
        }
        Err(e) => {
            console_log!("Safety queue: load archetype {slug} failed: {e:?}");
            return Err(());
        }
    };

    let verdict = classify_persona(env, &row.voice_prompt).await;

    match crate::storage::set_archetype_safety(&db, slug, &verdict).await {
        Ok(()) => {
            if let Ok(kv) = env.kv("KV") {
                let _ = crate::storage::invalidate_archetype_cache(&kv, slug).await;
            }
            Ok(())
        }
        Err(e) => {
            console_log!("Safety queue: write-back for archetype {slug} failed: {e:?}");
            Err(())
        }
    }
}
