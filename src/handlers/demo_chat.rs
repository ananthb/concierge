//! Public live-demo chat (`POST /api/demo/chat`).
//!
//! Backed by static small-business presets in `crate::prompt` with
//! production-parity validation, safety classification, canned-rule matching,
//! and AI generation.

use serde::{Deserialize, Serialize};
use worker::*;

use crate::ai;
use crate::api::demo::DEMO_MAX_USER_TURNS;
use crate::helpers::truncate;
use crate::prompt;
use crate::risk;
use crate::safety::{self, SafetyVerdict};

const MAX_BODY_BYTES: usize = 16_384;
const MAX_CONTENT_CHARS: usize = 300;
const RATE_LIMIT_PER_HOUR: i64 = 10;
const RATE_LIMIT_TTL_SECONDS: u64 = 3600;
const RATE_LIMIT_GLOBAL_PER_DAY: i64 = 2000;
const RATE_LIMIT_GLOBAL_TTL_SECONDS: u64 = 90_000;

#[derive(Deserialize)]
struct DemoChatReq {
    persona_id: String,
    #[serde(default)]
    business_name: Option<String>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    bot_prompt: Option<String>,
    #[serde(default)]
    match_rule: Option<String>,
    #[serde(default)]
    canned_text: Option<String>,
    #[serde(default)]
    history: Vec<HistoryTurn>,
    message: String,
    #[serde(default)]
    handoff: bool,
}

#[derive(Deserialize)]
struct HistoryTurn {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatReply {
    reply: String,
    handed_off: bool,
    matched_rule: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct ResolvedBotConfig {
    business_name: String,
    mode: String,
    bot_prompt: String,
    match_rule: String,
    canned_text: String,
}

fn resolve_and_validate_config(
    parsed: &DemoChatReq,
    persona: &prompt::BotPersona,
) -> std::result::Result<ResolvedBotConfig, &'static str> {
    let business_name = truncate(
        parsed
            .business_name
            .as_deref()
            .map(str::trim)
            .unwrap_or(persona.business_name),
        200,
    );
    let mode = parsed
        .mode
        .as_deref()
        .map(str::trim)
        .unwrap_or(persona.mode)
        .to_string();
    if mode != "ai" && mode != "canned" {
        return Err("Reply mode must be 'ai' or 'canned'.");
    }

    let bot_prompt = parsed
        .bot_prompt
        .as_deref()
        .map(str::trim)
        .unwrap_or(persona.prompt)
        .to_string();
    if bot_prompt.chars().count() > prompt::MAX_BOT_PROMPT {
        return Err("Bot prompt must be 600 characters or fewer.");
    }

    let match_rule = parsed
        .match_rule
        .as_deref()
        .map(str::trim)
        .unwrap_or(persona.match_rule)
        .to_string();
    if match_rule.chars().count() > prompt::MAX_MATCH_RULE {
        return Err("Match rule must be 300 characters or fewer.");
    }

    let canned_text = parsed
        .canned_text
        .as_deref()
        .map(str::trim)
        .unwrap_or(persona.canned_text)
        .to_string();
    if canned_text.chars().count() > prompt::MAX_CANNED_TEXT {
        return Err("Canned reply must be 2000 characters or fewer.");
    }
    if mode == "canned" && canned_text.is_empty() {
        return Err("Write the canned reply message.");
    }

    Ok(ResolvedBotConfig {
        business_name,
        mode,
        bot_prompt,
        match_rule,
        canned_text,
    })
}

fn user_edited_safety_text(
    parsed: &DemoChatReq,
    persona: &prompt::BotPersona,
    resolved: &ResolvedBotConfig,
) -> Option<String> {
    let candidates = [
        (
            parsed.business_name.is_some(),
            resolved.business_name.as_str(),
            persona.business_name,
        ),
        (
            parsed.bot_prompt.is_some(),
            resolved.bot_prompt.as_str(),
            persona.prompt,
        ),
        (
            parsed.match_rule.is_some(),
            resolved.match_rule.as_str(),
            persona.match_rule,
        ),
        (
            parsed.canned_text.is_some(),
            resolved.canned_text.as_str(),
            persona.canned_text,
        ),
    ];
    let edited: Vec<&str> = candidates
        .into_iter()
        .filter_map(|(provided, current, preset_val)| {
            if provided && !current.is_empty() && current != preset_val {
                Some(current)
            } else {
                None
            }
        })
        .collect();
    if edited.is_empty() {
        None
    } else {
        Some(edited.join("\n"))
    }
}

pub async fn handle_demo_chat(mut req: Request, env: Env) -> Result<Response> {
    let body = req.bytes().await.unwrap_or_default();
    if body.len() > MAX_BODY_BYTES {
        return crate::api::error("too_long", "Message too long.", 413);
    }

    let mut parsed: DemoChatReq = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(_) => return crate::api::invalid("Bad request body."),
    };

    let Some(persona) = prompt::find_bot_persona(&parsed.persona_id) else {
        return crate::api::invalid("Unknown demo persona.");
    };

    let resolved = match resolve_and_validate_config(&parsed, persona) {
        Ok(cfg) => cfg,
        Err(msg) => return crate::api::invalid(msg),
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

    let db = env.d1("DB")?;

    let now_iso = crate::helpers::now_iso();
    let date_key = now_iso.get(0..10).unwrap_or("1970-01-01");
    let global_key = format!("ratelimit:demo:global:{}", date_key);
    let global_count = crate::storage::get_rate_limit(&db, &global_key)
        .await
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
        let count = crate::storage::get_rate_limit(&db, &rl_key)
            .await
            .unwrap_or(0);
        if count >= RATE_LIMIT_PER_HOUR {
            return crate::api::error(
                "rate_limited",
                "You've sent quite a few messages — give me a minute and try again.",
                429,
            );
        }
        let _ =
            crate::storage::put_rate_limit(&db, &rl_key, count + 1, RATE_LIMIT_TTL_SECONDS).await;
    }

    let _ = crate::storage::put_rate_limit(
        &db,
        &global_key,
        global_count + 1,
        RATE_LIMIT_GLOBAL_TTL_SECONDS,
    )
    .await;

    if let Some(safety_text) = user_edited_safety_text(&parsed, persona, &resolved) {
        if let SafetyVerdict::Rejected { vague_reason } =
            safety::classify_text(&env, &safety_text).await
        {
            return crate::api::error("unsafe_content", &vague_reason, 422);
        }
    }

    if ai::is_prompt_injection(&env, &message).await {
        return crate::api::invalid(
            "We can't process that message. Please rephrase your question.",
        );
    }

    if !parsed.handoff && resolved.mode == "canned" {
        let matched = resolved.match_rule.is_empty()
            || ai::matches_canned_rule(&env, &message, &resolved.match_rule).await;
        if matched {
            return crate::api::json(&ChatReply {
                reply: resolved.canned_text,
                handed_off: false,
                matched_rule: true,
            });
        }
    }

    let system_prompt = if parsed.handoff {
        prompt::wrap(prompt::HOLDING_PATTERN_MIDDLE)
    } else {
        prompt::compose_bot_system_prompt(&resolved.business_name, &resolved.bot_prompt)
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
            console_log!("Demo chat AI error (persona={}): {:?}", persona.id, e);
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

    crate::api::json(&ChatReply {
        reply,
        handed_off,
        matched_rule: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_req(persona_id: &str) -> DemoChatReq {
        DemoChatReq {
            persona_id: persona_id.to_string(),
            business_name: None,
            mode: None,
            bot_prompt: None,
            match_rule: None,
            canned_text: None,
            history: vec![],
            message: "What are your hours?".to_string(),
            handoff: false,
        }
    }

    #[test]
    fn omitted_fields_fall_back_to_preset() {
        let persona = prompt::find_bot_persona("orders").unwrap();
        let req = base_req("orders");
        let resolved = resolve_and_validate_config(&req, persona).unwrap();
        assert_eq!(resolved.business_name, persona.business_name);
        assert_eq!(resolved.mode, "canned");
        assert_eq!(resolved.bot_prompt, persona.prompt);
        assert_eq!(resolved.match_rule, persona.match_rule);
        assert_eq!(resolved.canned_text, persona.canned_text);
        assert!(user_edited_safety_text(&req, persona, &resolved).is_none());
    }

    #[test]
    fn rejects_invalid_mode() {
        let persona = prompt::find_bot_persona("front-desk").unwrap();
        let mut req = base_req("front-desk");
        req.mode = Some("voice".to_string());
        assert_eq!(
            resolve_and_validate_config(&req, persona),
            Err("Reply mode must be 'ai' or 'canned'.")
        );
    }

    #[test]
    fn rejects_canned_mode_with_empty_canned_text() {
        let persona = prompt::find_bot_persona("front-desk").unwrap();
        let mut req = base_req("front-desk");
        req.mode = Some("canned".to_string());
        req.canned_text = Some("   ".to_string());
        assert_eq!(
            resolve_and_validate_config(&req, persona),
            Err("Write the canned reply message.")
        );
    }

    #[test]
    fn rejects_overlength_fields() {
        let persona = prompt::find_bot_persona("front-desk").unwrap();

        let mut req_prompt = base_req("front-desk");
        req_prompt.bot_prompt = Some("a".repeat(prompt::MAX_BOT_PROMPT + 1));
        assert_eq!(
            resolve_and_validate_config(&req_prompt, persona),
            Err("Bot prompt must be 600 characters or fewer.")
        );

        let mut req_rule = base_req("front-desk");
        req_rule.match_rule = Some("a".repeat(prompt::MAX_MATCH_RULE + 1));
        assert_eq!(
            resolve_and_validate_config(&req_rule, persona),
            Err("Match rule must be 300 characters or fewer.")
        );

        let mut req_canned = base_req("front-desk");
        req_canned.canned_text = Some("a".repeat(prompt::MAX_CANNED_TEXT + 1));
        assert_eq!(
            resolve_and_validate_config(&req_canned, persona),
            Err("Canned reply must be 2000 characters or fewer.")
        );
    }

    #[test]
    fn collects_only_edited_fields_for_safety() {
        let persona = prompt::find_bot_persona("front-desk").unwrap();
        let mut req = base_req("front-desk");
        req.business_name = Some(persona.business_name.to_string());
        req.bot_prompt = Some("Custom bakery instructions".to_string());
        let resolved = resolve_and_validate_config(&req, persona).unwrap();
        assert_eq!(
            user_edited_safety_text(&req, persona, &resolved).as_deref(),
            Some("Custom bakery instructions")
        );
    }
}
