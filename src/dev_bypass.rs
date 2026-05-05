//! Local-dev bypass switches for the management panel and the AI
//! bindings that back it.
//!
//! There is exactly one trigger — the `MANAGE_BYPASS_EMAIL` env var
//! must be non-empty AND `CF_ACCESS_AUD` must be empty. Production
//! sets `CF_ACCESS_AUD` via the Cloudflare Workers build env, so the
//! bypass is impossible to activate in prod even if the email var
//! leaks: the AUD presence wins.
//!
//! When the bypass is active:
//!
//!   * `verify_access` (in `management::mod`) returns the bypass email
//!     instead of going through Cloudflare Access JWT verification.
//!     This is what lets a developer reach `/manage` from
//!     `wrangler dev` and what lets the Playwright suite drive the
//!     management panel.
//!   * The AI helpers in `crate::ai` short-circuit to canned stub
//!     responses instead of calling `env.ai("AI")`. `wrangler dev
//!     --local` doesn't proxy the remote AI binding, so without
//!     stubbing the demo persona generator + safety classifier would
//!     fail and the management panel UI would be broken on dev.
//!
//! The stub responses are deliberately *shaped* like real model output
//! so the rendered UI looks plausible, but they're constant — no
//! randomness, no actual model knowledge.

use worker::*;

/// True iff both bypass conditions are met. Cheap to call repeatedly.
pub fn active(env: &Env) -> bool {
    aud_empty(env) && bypass_email_set(env)
}

/// Read the bypass operator email when the bypass is active. Returns
/// `None` otherwise.
pub fn manage_bypass_email(env: &Env) -> Option<String> {
    if !aud_empty(env) {
        return None;
    }
    env.var("MANAGE_BYPASS_EMAIL")
        .ok()
        .map(|v| v.to_string())
        .filter(|s| !s.is_empty())
}

fn aud_empty(env: &Env) -> bool {
    match env.var("CF_ACCESS_AUD") {
        Ok(v) => v.to_string().is_empty(),
        Err(_) => true,
    }
}

fn bypass_email_set(env: &Env) -> bool {
    matches!(env.var("MANAGE_BYPASS_EMAIL"), Ok(v) if !v.to_string().is_empty())
}

/// Canned shape for `crate::ai::generate_persona_businesses`. Returns
/// a JSON array of N entries (one per archetype) matching the schema
/// the persona generator expects. The names rotate through a small
/// fixture pool so each archetype gets a different-looking business
/// without being random.
pub fn stub_persona_businesses(archetype_count: usize) -> String {
    const FIXTURES: &[(&str, &str, &str, &str, &str, &str)] = &[
        (
            "Aurelia Tea Co.",
            "boutique tea shop",
            "Brooklyn",
            "10am–7pm Tue–Sun",
            "book a tasting flight",
            "https://example.com/book",
        ),
        (
            "North Loop Dental",
            "family dentistry",
            "Minneapolis",
            "8am–5pm Mon–Fri",
            "schedule a cleaning",
            "https://example.com/dental",
        ),
        (
            "Verdigris Studio",
            "ceramics studio",
            "Portland",
            "by appointment",
            "register for a class",
            "https://example.com/classes",
        ),
        (
            "Coast & Pine Hotel",
            "boutique hotel",
            "Half Moon Bay",
            "front desk 24/7",
            "reserve an ocean-view room",
            "https://example.com/rooms",
        ),
        (
            "Field & Vine Catering",
            "small-event catering",
            "Austin",
            "Mon–Sat",
            "request a catering quote",
            "https://example.com/quote",
        ),
        (
            "Rivermouth Auto",
            "auto repair shop",
            "Sacramento",
            "7am–6pm Mon–Sat",
            "book a service appointment",
            "https://example.com/service",
        ),
    ];
    let n = archetype_count.max(1);
    let entries: Vec<serde_json::Value> = (0..n)
        .map(|i| {
            let f = FIXTURES[i % FIXTURES.len()];
            serde_json::json!({
                "name": f.0,
                "business_type": f.1,
                "city": f.2,
                "hours": f.3,
                "goal": f.4,
                "goal_url": f.5,
            })
        })
        .collect();
    serde_json::Value::Array(entries).to_string()
}

/// Canned chat reply for `crate::ai::generate_response` when the
/// bypass is active. Operators flipping the demo on without remote AI
/// access still see a plausible reply instead of an error toast.
pub const STUB_CHAT_REPLY: &str =
    "Thanks for reaching out — happy to help. (dev bypass: this is a stub reply.)";

/// Stub embedding vector for `crate::ai::embed`. Real BGE returns a
/// 768-dimensional vector; we return a deterministic one so any rule
/// matching done downstream sees stable values.
pub fn stub_embedding() -> Vec<f32> {
    // 768-dim vector of small constants. Not zero (some downstream
    // code may special-case zero magnitude); not normalized either —
    // dev only.
    (0..768).map(|i| ((i % 17) as f32) * 0.01).collect()
}
