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
use crate::types::StaticRule;

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
    bot_prompt: Option<String>,
    #[serde(default)]
    rules: Option<Vec<StaticRule>>,
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
    bot_prompt: String,
    rules: Vec<StaticRule>,
}

fn preset_rules(persona: &prompt::BotPersona) -> Vec<StaticRule> {
    persona
        .rules
        .iter()
        .map(|r| StaticRule {
            match_rule: r.match_rule.to_string(),
            reply_text: r.reply_text.to_string(),
        })
        .collect()
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

    let bot_prompt = parsed
        .bot_prompt
        .as_deref()
        .map(str::trim)
        .unwrap_or(persona.prompt)
        .to_string();
    if bot_prompt.chars().count() > prompt::MAX_BOT_PROMPT {
        return Err("Bot prompt must be 600 characters or fewer.");
    }

    let rules = match parsed.rules.as_ref() {
        Some(raw) => prompt::validate_static_rules(raw)?,
        None => preset_rules(persona),
    };

    Ok(ResolvedBotConfig {
        business_name,
        bot_prompt,
        rules,
    })
}

fn user_edited_safety_text(
    parsed: &DemoChatReq,
    persona: &prompt::BotPersona,
    resolved: &ResolvedBotConfig,
) -> Option<String> {
    let mut edited: Vec<&str> = Vec::new();
    if parsed.business_name.is_some()
        && !resolved.business_name.is_empty()
        && resolved.business_name != persona.business_name
    {
        edited.push(&resolved.business_name);
    }
    if parsed.bot_prompt.is_some()
        && !resolved.bot_prompt.is_empty()
        && resolved.bot_prompt != persona.prompt
    {
        edited.push(&resolved.bot_prompt);
    }
    if parsed.rules.is_some() && resolved.rules != preset_rules(persona) {
        for r in &resolved.rules {
            edited.push(&r.match_rule);
            edited.push(&r.reply_text);
        }
    }
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
            crate::metrics::record(&env, "demo", "safety_rejected", persona.id);
            return crate::api::error("unsafe_content", &vague_reason, 422);
        }
    }

    match ai::check_prompt_injection(&env, &message).await {
        ai::InjectionVerdict::Clean => {}
        ai::InjectionVerdict::Blocked { reason } => {
            crate::metrics::record(&env, "demo", "prompt_injection_blocked", reason);
            return crate::api::invalid(
                "We can't process that message. Please rephrase your question.",
            );
        }
        ai::InjectionVerdict::Error => {
            crate::metrics::record(&env, "demo", "prompt_guard_error", "ai_error");
        }
    }

    if !parsed.handoff && !resolved.rules.is_empty() {
        if let Some(idx) = ai::pick_static_rule(&env, &message, &resolved.rules).await {
            crate::metrics::record(&env, "demo", "reply_sent", "rule");
            return crate::api::json(&ChatReply {
                reply: resolved.rules[idx].reply_text.clone(),
                handed_off: false,
                matched_rule: true,
            });
        }
    }

    let system_prompt = if parsed.handoff {
        prompt::wrap(prompt::HOLDING_PATTERN_MIDDLE)
    } else {
        prompt::compose_bot_system_prompt(&resolved.bot_prompt)
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

    crate::metrics::record(
        &env,
        "demo",
        "reply_sent",
        if handed_off { "ai_handoff" } else { "ai" },
    );
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
            bot_prompt: None,
            rules: None,
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
        assert_eq!(resolved.bot_prompt, persona.prompt);
        assert_eq!(resolved.rules, preset_rules(persona));
        assert_eq!(resolved.rules.len(), 2);
        assert!(user_edited_safety_text(&req, persona, &resolved).is_none());
    }

    #[test]
    fn rejects_empty_rule_fields_and_too_many_rules() {
        let persona = prompt::find_bot_persona("front-desk").unwrap();

        let mut req_empty_match = base_req("front-desk");
        req_empty_match.rules = Some(vec![StaticRule {
            match_rule: "   ".to_string(),
            reply_text: "Open 9-5".to_string(),
        }]);
        assert_eq!(
            resolve_and_validate_config(&req_empty_match, persona),
            Err("Each rule needs a trigger condition.")
        );

        let mut req_empty_reply = base_req("front-desk");
        req_empty_reply.rules = Some(vec![StaticRule {
            match_rule: "hours".to_string(),
            reply_text: "   ".to_string(),
        }]);
        assert_eq!(
            resolve_and_validate_config(&req_empty_reply, persona),
            Err("Each rule needs a reply message.")
        );

        let mut req_too_many = base_req("front-desk");
        req_too_many.rules = Some(
            (0..=prompt::MAX_STATIC_RULES)
                .map(|i| StaticRule {
                    match_rule: format!("rule {i}"),
                    reply_text: format!("reply {i}"),
                })
                .collect(),
        );
        assert_eq!(
            resolve_and_validate_config(&req_too_many, persona),
            Err("You can add up to 10 static rules.")
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
        req_rule.rules = Some(vec![StaticRule {
            match_rule: "a".repeat(prompt::MAX_MATCH_RULE + 1),
            reply_text: "ok".to_string(),
        }]);
        assert_eq!(
            resolve_and_validate_config(&req_rule, persona),
            Err("Match rule must be 300 characters or fewer.")
        );

        let mut req_canned = base_req("front-desk");
        req_canned.rules = Some(vec![StaticRule {
            match_rule: "hours".to_string(),
            reply_text: "a".repeat(prompt::MAX_CANNED_TEXT + 1),
        }]);
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
