//! Public live-demo chat. Reachable from the welcome page CTA; the
//! visitor picks one of the personas exposed by `crate::demo_personas`
//! (Concierge talking about itself, or one of the four tenant presets)
//! and the worker forwards their message history to Cloudflare Workers
//! AI under that persona's system prompt. Per-IP rate-limited so an
//! unauth public endpoint can't be abused.

use serde::{Deserialize, Serialize};
use worker::*;

use crate::ai;
use crate::demo_personas;

const MAX_TURNS: usize = 12;
const MAX_BODY_BYTES: usize = 4096;
const MAX_CONTENT_CHARS: usize = 600;
const RATE_LIMIT_PER_HOUR: i64 = 30;
const RATE_LIMIT_TTL_SECONDS: u64 = 3600;

#[derive(Deserialize)]
struct ChatRequest {
    #[serde(default)]
    persona: String,
    messages: Vec<ChatMessage>,
}

#[derive(Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatReply<'a> {
    reply: &'a str,
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
    // turn — leading with assistant content (e.g. a client-side greeting)
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

    let client_ip = req
        .headers()
        .get("CF-Connecting-IP")
        .ok()
        .flatten()
        .unwrap_or_default();
    if !client_ip.is_empty() {
        let kv = env.kv("KV")?;
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

    let persona = demo_personas::lookup(&parsed.persona);
    // Same envelope every tenant prompt gets — the demo is just another
    // small business in the wrapper's eyes.
    let wrapped_prompt = crate::prompt::wrap(persona.prompt);
    let history: Vec<(String, String)> = parsed
        .messages
        .into_iter()
        .map(|m| (m.role, m.content))
        .collect();

    let reply = match ai::generate_chat_reply(&env, &wrapped_prompt, &history).await {
        Ok(r) => r.trim().to_string(),
        Err(e) => {
            console_log!("Demo chat AI error (persona={}): {:?}", persona.slug, e);
            return json_error("Couldn't generate a reply right now.", 502);
        }
    };

    json_response(&ChatReply { reply: &reply }, 200)
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
