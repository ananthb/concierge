//! Public live-demo chat (`POST /api/demo/chat`).
//!
//! Backed by static small-business fixtures in `crate::api::demo` and the
//! single core small-business persona in `crate::prompt`.

use serde::{Deserialize, Serialize};
use worker::*;

use crate::ai;
use crate::api::demo::{find_fixture, DEMO_MAX_USER_TURNS};
use crate::prompt;
use crate::risk;

const MAX_BODY_BYTES: usize = 4096;
const MAX_CONTENT_CHARS: usize = 300;
const RATE_LIMIT_PER_HOUR: i64 = 10;
const RATE_LIMIT_TTL_SECONDS: u64 = 3600;
const RATE_LIMIT_GLOBAL_PER_DAY: i64 = 2000;
const RATE_LIMIT_GLOBAL_TTL_SECONDS: u64 = 90_000;

#[derive(Deserialize)]
struct ChatRequest {
    persona_id: String,
    #[serde(default)]
    history: Vec<ChatMessage>,
    message: String,
    #[serde(default)]
    handoff: bool,
}

#[derive(Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatReply {
    reply: String,
    handed_off: bool,
}

pub async fn handle_demo_chat(mut req: Request, env: Env) -> Result<Response> {
    let body = req.bytes().await.unwrap_or_default();
    if body.len() > MAX_BODY_BYTES {
        return crate::api::error("too_long", "Message too long.", 413);
    }

    let mut parsed: ChatRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(_) => return crate::api::invalid("Bad request body."),
    };

    let Some(fixture) = find_fixture(&parsed.persona_id) else {
        return crate::api::invalid("Unknown demo business.");
    };

    let message = parsed.message.trim().to_string();
    if message.is_empty() {
        return crate::api::invalid("Write a message to try the demo.");
    }
    if message.chars().count() > MAX_CONTENT_CHARS {
        return crate::api::invalid("Message too long.");
    }

    for m in &parsed.history {
        if m.role != "user" && m.role != "assistant" {
            return crate::api::invalid("Invalid message role.");
        }
        if m.content.chars().count() > MAX_CONTENT_CHARS {
            return crate::api::invalid("Message too long.");
        }
    }

    while parsed.history.first().map(|m| m.role.as_str()) == Some("assistant") {
        parsed.history.remove(0);
    }

    let prior_user_turns = parsed.history.iter().filter(|m| m.role == "user").count();
    if prior_user_turns + 1 > DEMO_MAX_USER_TURNS as usize
        || parsed.history.len() >= (DEMO_MAX_USER_TURNS as usize) * 2
    {
        return crate::api::invalid("Too many turns; refresh and start over.");
    }

    let kv = env.kv("KV")?;

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
        return crate::api::error(
            "rate_limited",
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
            return crate::api::error(
                "rate_limited",
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

    let _ = kv
        .put(&global_key, (global_count + 1).to_string())?
        .expiration_ttl(RATE_LIMIT_GLOBAL_TTL_SECONDS)
        .execute()
        .await;

    if ai::is_prompt_injection(&env, &message).await {
        return crate::api::invalid(
            "We can't process that message. Please rephrase your question.",
        );
    }

    let system_prompt = if parsed.handoff {
        prompt::wrap(prompt::HOLDING_PATTERN_MIDDLE)
    } else {
        prompt::demo_system_prompt(
            fixture.business_name,
            fixture.business_type,
            fixture.city,
            fixture.hours,
            fixture.summary,
        )
    };

    let mut chat_history: Vec<(String, String)> = parsed
        .history
        .into_iter()
        .map(|m| (m.role, m.content))
        .collect();
    chat_history.push(("user".to_string(), message));

    let raw_reply = match ai::generate_chat_reply(&env, &system_prompt, &chat_history).await {
        Ok(r) => r,
        Err(e) => {
            console_log!("Demo chat AI error (persona={}): {:?}", fixture.id, e);
            return crate::api::error("ai_error", "Couldn't generate a reply right now.", 502);
        }
    };

    let stripped = prompt::detect_and_strip_handoff(&raw_reply);
    let mut reply = stripped.reply;
    let mut handed_off = stripped.handoff || parsed.handoff;

    if !handed_off && risk::signal(&reply).is_some() {
        reply = prompt::RISK_GATE_HOLDING_REPLY.to_string();
        handed_off = true;
    }

    crate::api::json(&ChatReply { reply, handed_off })
}
