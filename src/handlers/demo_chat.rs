//! Public live-demo chat. Reachable from the welcome page CTA; the
//! visitor picks one of the personas exposed by the D1 catalog
//! (Concierge talking about itself, plus management-curated archetypes)
//! and the worker forwards their message history to Cloudflare Workers
//! AI under that persona's system prompt. Two-tier rate limiting: a
//! per-IP hourly cap, plus a global daily cap (per UTC day, all IPs)
//! that bounds worst-case Workers AI spend even under IP rotation.
//! Every reply also runs through the prompt-injection scanner and the
//! Approved-only persona gate. No bypass.
//!
//! Handoff: the demo handler is stateless on the server. The client
//! tracks `handoff` in modal state; once set, every subsequent send
//! includes `handoff: true` and the server swaps the persona middle
//! for [`crate::prompt::HOLDING_PATTERN_MIDDLE`]. There's no real
//! human to escalate to in the demo, so the holding pattern lasts as
//! long as the modal session does. There's no cooldown branch.

use serde::{Deserialize, Serialize};
use worker::*;

use crate::ai;
use crate::storage;

const MAX_TURNS: usize = 6;
const MAX_BODY_BYTES: usize = 4096;
const MAX_CONTENT_CHARS: usize = 300;
const RATE_LIMIT_PER_HOUR: i64 = 10;
const RATE_LIMIT_TTL_SECONDS: u64 = 3600;
// Global daily ceiling across all IPs. Bounds worst-case Workers AI
// spend regardless of how many distinct sources hit the demo (per-IP
// limits don't help against IP rotation). At ~$0.002/request worst
// case this caps daily exposure around a few dollars.
const RATE_LIMIT_GLOBAL_PER_DAY: i64 = 2000;
// Slightly over 24h so a counter that gets created near midnight UTC
// still expires cleanly the following day.
const RATE_LIMIT_GLOBAL_TTL_SECONDS: u64 = 90_000;

#[derive(Deserialize)]
struct ChatRequest {
    #[serde(default)]
    persona: String,
    messages: Vec<ChatMessage>,
    /// Set by the client once a previous turn flagged a handoff. When
    /// true the server replies under the holding-pattern middle, not
    /// the persona's prompt.
    #[serde(default)]
    handoff: bool,
}

#[derive(Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatReply<'a> {
    reply: &'a str,
    /// True iff the AI's reply contained the handoff token. The token
    /// is stripped from `reply` before sending; the client uses this
    /// flag to flip into the holding-pattern UI and to echo
    /// `handoff: true` on subsequent sends.
    handoff: bool,
}

#[derive(Serialize)]
struct ChatError<'a> {
    error: &'a str,
}

pub async fn handle_demo_chat(mut req: Request, env: Env) -> Result<Response> {
    let body = req.bytes().await.unwrap_or_default();
    if body.len() > MAX_BODY_BYTES {
        return json_error("Message too long.", 413);
    }

    let mut parsed: ChatRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(_) => return json_error("Bad request body.", 400),
    };

    if parsed.messages.is_empty() {
        return json_error("No messages.", 400);
    }
    if parsed.messages.len() > MAX_TURNS {
        return json_error("Too many turns; refresh and start over.", 400);
    }
    for m in &parsed.messages {
        if m.role != "user" && m.role != "assistant" {
            return json_error("Invalid message role.", 400);
        }
        if m.content.chars().count() > MAX_CONTENT_CHARS {
            return json_error("Message too long.", 400);
        }
    }
    // Llama chat templates expect the first non-system message to be a user
    // turn. Leading with assistant content (e.g. a client-side greeting)
    // confuses generation. Strip any leading assistant messages here.
    while parsed.messages.first().map(|m| m.role.as_str()) == Some("assistant") {
        parsed.messages.remove(0);
    }
    if parsed.messages.is_empty() {
        return json_error("No user messages.", 400);
    }
    if parsed.messages.last().map(|m| m.role.as_str()) != Some("user") {
        return json_error("Last message must be from the user.", 400);
    }

    let kv = env.kv("KV")?;

    // Global daily cap. Checked first so an exhausted day refuses
    // immediately, regardless of whether we have a client IP.
    let now_iso = crate::helpers::now_iso();
    let date_key = now_iso.get(0..10).unwrap_or("1970-01-01");
    let global_key = format!("ratelimit:demo:global:{}", date_key);
    let global_count: i64 = kv
        .get(&global_key)
        .text()
        .await
        .ok()
        .flatten()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if global_count >= RATE_LIMIT_GLOBAL_PER_DAY {
        return json_error(
            "The demo is at capacity for today. Please try again tomorrow.",
            429,
        );
    }

    let client_ip = req
        .headers()
        .get("CF-Connecting-IP")
        .ok()
        .flatten()
        .unwrap_or_default();
    if !client_ip.is_empty() {
        let rl_key = format!("ratelimit:demo:{}", client_ip);
        let count: i64 = kv
            .get(&rl_key)
            .text()
            .await
            .ok()
            .flatten()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if count >= RATE_LIMIT_PER_HOUR {
            return json_error(
                "You've sent quite a few messages — give me a minute and try again.",
                429,
            );
        }
        let _ = kv
            .put(&rl_key, (count + 1).to_string())?
            .expiration_ttl(RATE_LIMIT_TTL_SECONDS)
            .execute()
            .await;
    }

    // Bump the global counter only after the per-IP check passes, so a
    // caller already over their personal limit doesn't drain the shared
    // budget on requests we're going to refuse anyway.
    let _ = kv
        .put(&global_key, (global_count + 1).to_string())?
        .expiration_ttl(RATE_LIMIT_GLOBAL_TTL_SECONDS)
        .execute()
        .await;

    // Resolve the persona from the D1 catalog. Refuse with 503 if the
    // requested slug isn't there or isn't Approved. No bypass even if a
    // management user is testing.
    let db = env.d1("DB")?;
    let slug = if parsed.persona.is_empty() {
        crate::storage::DEMO_DEFAULT_PERSONA_SLUG.to_string()
    } else {
        parsed.persona.clone()
    };
    let row = if slug == crate::storage::DEMO_DEFAULT_PERSONA_SLUG {
        None
    } else {
        match storage::get_archetype_cached(&kv, &db, &slug).await? {
            Some(r) if r.is_safe_to_use() => Some(r),
            _ => {
                return json_error(
                    "That persona isn't available right now. Please pick another.",
                    503,
                );
            }
        }
    };

    // Run the prompt-injection scanner on the visitor's last user turn
    // before we spend any tokens or KV credit.
    let last_user = parsed
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.as_str())
        .unwrap_or("");
    if ai::is_prompt_injection(&env, last_user).await {
        return json_error(
            "We can't process that message. Please rephrase your question.",
            400,
        );
    }

    // Resolve the persona's prompt. In handoff mode the persona is
    // replaced wholesale by [`crate::prompt::HOLDING_PATTERN_MIDDLE`]
    // (no demo frame, no goal-driving) until the modal session ends.
    let wrapped_prompt = if parsed.handoff {
        crate::prompt::wrap(crate::prompt::HOLDING_PATTERN_MIDDLE)
    } else {
        let persona_middle = if slug == crate::storage::DEMO_DEFAULT_PERSONA_SLUG {
            crate::prompt::CONCIERGE_PROMPT.to_string()
        } else if let Some(r) = row {
            // For demo purposes, we need a PersonaBuilder with fictional details.
            // The /demo/personas endpoint provides these (cached).
            // If they hit /demo/chat directly with a slug, we just use empty defaults
            // for the builder and the archetype's voice.
            crate::personas::generate(&crate::types::PersonaBuilder::default(), &r.voice_prompt)
        } else {
            String::new()
        };
        let demo_middle = crate::prompt::compose_demo_middle(&persona_middle, &slug);
        crate::prompt::wrap(&demo_middle)
    };

    let history: Vec<(String, String)> = parsed
        .messages
        .into_iter()
        .map(|m| (m.role, m.content))
        .collect();

    let raw_reply = match ai::generate_chat_reply(&env, &wrapped_prompt, &history).await {
        Ok(r) => r,
        Err(e) => {
            console_log!("Demo chat AI error (persona={}): {:?}", slug, e);
            return json_error("Couldn't generate a reply right now.", 502);
        }
    };

    // Scan for the handoff sentinel before the customer ever sees it.
    // If we're already in the holding pattern, the postamble told the
    // model not to re-emit; this just defends against the model
    // ignoring that instruction.
    let stripped = crate::prompt::detect_and_strip_handoff(&raw_reply);
    let handoff_signaled = stripped.handoff && !parsed.handoff;
    json_response(
        &ChatReply {
            reply: &stripped.reply,
            handoff: handoff_signaled || parsed.handoff,
        },
        200,
    )
}

fn json_response<T: Serialize>(body: &T, status: u16) -> Result<Response> {
    let serialized = serde_json::to_string(body)
        .map_err(|e| Error::from(format!("Failed to serialize chat response: {}", e)))?;
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Cache-Control", "no-store")?;
    Ok(Response::ok(serialized)?
        .with_status(status)
        .with_headers(headers))
}

fn json_error(msg: &str, status: u16) -> Result<Response> {
    json_response(&ChatError { error: msg }, status)
}
