use serde::{Deserialize, Serialize};
use worker::*;

const DEFAULT_MODEL: &str = "@cf/meta/llama-4-scout-17b-16e-instruct";
const DEFAULT_INJECTION_MODEL: &str = "@cf/meta/prompt-guard-2-86m";
pub const DEFAULT_DECISION_MODEL: &str = "@cf/meta/llama-3.1-8b-instruct-fast";

fn get_model(env: &Env) -> String {
    env.var("AI_MODEL")
        .ok()
        .map(|v| v.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

fn get_injection_model(env: &Env) -> String {
    env.var("AI_INJECTION_MODEL")
        .ok()
        .map(|v| v.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_INJECTION_MODEL.to_string())
}

fn get_decision_model(env: &Env) -> String {
    env.var("AI_DECISION_MODEL")
        .ok()
        .map(|v| v.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_DECISION_MODEL.to_string())
}

#[derive(Serialize)]
struct AiRequest {
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct PromptGuardRequest {
    text: String,
}

// ============================================================================
// AI Response Generation
// ============================================================================

/// Generate a multi-turn chat reply. The caller passes the actual
/// `(role, content)` history and we forward it verbatim.
pub async fn generate_chat_reply(
    env: &Env,
    system_prompt: &str,
    history: &[(String, String)],
) -> Result<String> {
    if crate::dev_bypass::active(env) {
        return Ok(crate::dev_bypass::STUB_CHAT_REPLY.to_string());
    }
    let mut messages = Vec::with_capacity(history.len() + 1);
    messages.push(Message {
        role: "system".to_string(),
        content: system_prompt.to_string(),
    });
    for (role, content) in history {
        messages.push(Message {
            role: role.clone(),
            content: content.clone(),
        });
    }
    let request = AiRequest {
        messages,
        max_tokens: None,
    };
    let model = get_model(env);
    let reply = run_ai_model(env, &model, &request).await?;
    let trimmed = reply.trim();
    if trimmed.is_empty() {
        Err(Error::from("AI returned empty chat response"))
    } else {
        Ok(trimmed.to_string())
    }
}

// ============================================================================
// Prompt Injection Detection (@cf/meta/prompt-guard-2-86m)
// ============================================================================

/// Check if a message looks like a prompt injection or jailbreak attempt.
/// Returns `true` if injection is detected. Fails closed (`true` on error).
pub async fn is_prompt_injection(env: &Env, text: &str) -> bool {
    if crate::dev_bypass::active(env) || text.trim().len() < 10 {
        return false;
    }

    let model = get_injection_model(env);
    let ai = match env.ai("AI") {
        Ok(a) => a,
        Err(_) => return true,
    };

    let req = PromptGuardRequest {
        text: text.to_string(),
    };
    let result: std::result::Result<serde_json::Value, _> = ai.run(&model, &req).await;
    match result {
        Ok(val) => parse_prompt_guard_verdict(&val),
        Err(e) => {
            console_log!("Prompt guard error: {:?}", e);
            true
        }
    }
}

/// Parse Cloudflare Workers AI `@cf/meta/prompt-guard-2-86m` classification
/// output. Returns `true` (injection detected / fail-closed) if any entry has a
/// malicious/injection/jailbreak label with `score >= 0.5`, OR if no recognized
/// benign/malicious label is present.
pub(crate) fn parse_prompt_guard_verdict(val: &serde_json::Value) -> bool {
    let items: Vec<&serde_json::Value> = if let Some(arr) = val.as_array() {
        arr.iter().collect()
    } else if let Some(arr) = val.get("result").and_then(|v| v.as_array()) {
        arr.iter().collect()
    } else if let Some(arr) = val.get("response").and_then(|v| v.as_array()) {
        arr.iter().collect()
    } else if let Some(obj) = val.get("result").filter(|v| v.is_object()) {
        vec![obj]
    } else if let Some(obj) = val.get("response").filter(|v| v.is_object()) {
        vec![obj]
    } else if val.is_object() {
        vec![val]
    } else {
        return true;
    };

    let mut saw_recognized = false;
    for item in items {
        let Some(label) = item.get("label").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(score) = item.get("score").and_then(|v| v.as_f64()) else {
            continue;
        };
        let upper = label.trim().to_ascii_uppercase();
        if upper.contains("MALICIOUS") || upper.contains("INJECTION") || upper.contains("JAILBREAK")
        {
            saw_recognized = true;
            if score >= 0.5 {
                return true;
            }
        } else if upper == "BENIGN" || upper == "SAFE" {
            saw_recognized = true;
        }
    }

    !saw_recognized
}

// ============================================================================
// Static Rule Picker Decision (@cf/meta/llama-3.1-8b-instruct-fast)
// ============================================================================

const PICK_RULE_SYSTEM: &str = "You are a strict intent classifier. Given a numbered list of static auto-reply rules and an inbound customer message, decide whether the customer's message clearly and unambiguously matches one of the rules.

Rules:
- Err on the side of returning null. Only pick a rule_index when you are highly confident the message fits that specific rule.
- Respond ONLY with a single JSON object: {\"rule_index\": <integer or null>, \"confidence\": 0.0..1.0}.
- Output no markdown, no code fences, and no extra text.";

#[derive(Deserialize)]
struct RulePickDecision {
    rule_index: Option<usize>,
    confidence: f64,
}

/// Pick the index of the static rule that `user_message` clearly satisfies, or
/// `None` to fall back to AI generation.
///
/// - Empty `rules` or `DEV_MODE` stub mode always returns `None`.
/// - On AI error or uncertain/malformed output, returns `None`.
pub async fn pick_static_rule(
    env: &Env,
    user_message: &str,
    rules: &[crate::types::StaticRule],
) -> Option<usize> {
    if rules.is_empty() || crate::dev_bypass::active(env) {
        return None;
    }

    let numbered_rules = rules
        .iter()
        .enumerate()
        .map(|(i, r)| format!("{}: {}", i, r.match_rule.trim()))
        .collect::<Vec<_>>()
        .join("\n");

    let request = AiRequest {
        messages: vec![
            Message {
                role: "system".to_string(),
                content: PICK_RULE_SYSTEM.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: format!(
                    "Rules:\n{}\n\nCustomer message: {}",
                    numbered_rules, user_message
                ),
            },
        ],
        max_tokens: Some(64),
    };
    let model = get_decision_model(env);
    match run_ai_model(env, &model, &request).await {
        Ok(raw) => parse_rule_pick(&raw, rules.len()),
        Err(e) => {
            console_log!("Static rule picker error: {:?}", e);
            None
        }
    }
}

/// Parse the decision model's JSON response (`{"rule_index": <int or null>, "confidence": f64}`).
/// Returns `Some(idx)` ONLY when `rule_index == Some(idx)` with `idx < rule_count`
/// and `confidence.is_finite() && confidence >= 0.85`.
pub fn parse_rule_pick(raw: &str, rule_count: usize) -> Option<usize> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if start >= end {
        return None;
    }
    let slice = &raw[start..=end];
    let d = serde_json::from_str::<RulePickDecision>(slice).ok()?;
    let idx = d.rule_index?;
    if idx < rule_count && d.confidence.is_finite() && d.confidence >= 0.85 {
        Some(idx)
    } else {
        None
    }
}

// ============================================================================
// Internal
// ============================================================================

async fn run_ai_model(env: &Env, model: &str, request: &AiRequest) -> Result<String> {
    let ai = env.ai("AI")?;

    // Pass the typed `AiRequest` struct directly so `serde_wasm_bindgen`
    // emits a plain JS Object rather than a Map.
    let response: serde_json::Value = ai
        .run(model, request)
        .await
        .map_err(|e| Error::from(format!("AI model error: {:?}", e)))?;

    if let Some(s) = response.as_str() {
        return Ok(s.to_string());
    }
    if let Some(s) = response.get("response").and_then(|r| r.as_str()) {
        return Ok(s.to_string());
    }
    if let Some(s) = response
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c0| c0.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
    {
        return Ok(s.to_string());
    }
    if let Some(s) = response.get("output_text").and_then(|s| s.as_str()) {
        return Ok(s.to_string());
    }
    if let Some(s) = response
        .get("result")
        .and_then(|r| r.get("response"))
        .and_then(|r| r.as_str())
    {
        return Ok(s.to_string());
    }

    console_log!(
        "AI: unknown response shape from {}: {}",
        model,
        response.to_string().chars().take(500).collect::<String>()
    );
    Err(Error::from(format!(
        "AI model {model} returned unrecognized response shape"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prompt_guard_benign_passes() {
        let val = json!([
            { "label": "BENIGN", "score": 0.99 },
            { "label": "MALICIOUS", "score": 0.01 }
        ]);
        assert!(!parse_prompt_guard_verdict(&val));
    }

    #[test]
    fn prompt_guard_malicious_or_injection_or_jailbreak_blocks() {
        for label in ["MALICIOUS", "INJECTION", "JAILBREAK", "malicious"] {
            let val = json!([
                { "label": "BENIGN", "score": 0.1 },
                { "label": label, "score": 0.5 }
            ]);
            assert!(
                parse_prompt_guard_verdict(&val),
                "expected injection for {label}"
            );
        }
    }

    #[test]
    fn prompt_guard_handles_result_and_response_wrappers() {
        assert!(!parse_prompt_guard_verdict(&json!({
            "result": [{ "label": "BENIGN", "score": 0.98 }]
        })));
        assert!(parse_prompt_guard_verdict(&json!({
            "response": [{ "label": "INJECTION", "score": 0.87 }]
        })));
    }

    #[test]
    fn prompt_guard_fails_closed_on_unrecognized_shapes() {
        assert!(parse_prompt_guard_verdict(&json!(null)));
        assert!(parse_prompt_guard_verdict(&json!([])));
        assert!(parse_prompt_guard_verdict(&json!({})));
        assert!(parse_prompt_guard_verdict(&json!([
            { "label": "UNKNOWN", "score": 0.99 }
        ])));
    }

    #[test]
    fn rule_pick_accepts_high_confidence_in_bounds() {
        assert_eq!(
            parse_rule_pick(r#"{"rule_index": 0, "confidence": 0.9}"#, 2),
            Some(0)
        );
        assert_eq!(
            parse_rule_pick(r#"{"rule_index": 1, "confidence": 0.85}"#, 2),
            Some(1)
        );
        assert_eq!(
            parse_rule_pick(
                "Sure! ```json\n{\"rule_index\": 0, \"confidence\": 0.96}\n```",
                3
            ),
            Some(0)
        );
    }

    #[test]
    fn rule_pick_rejects_low_confidence_null_or_oob() {
        assert_eq!(
            parse_rule_pick(r#"{"rule_index": 1, "confidence": 0.84}"#, 2),
            None
        );
        assert_eq!(
            parse_rule_pick(r#"{"rule_index": null, "confidence": 0.99}"#, 2),
            None
        );
        assert_eq!(
            parse_rule_pick(r#"{"rule_index": 5, "confidence": 0.95}"#, 2),
            None
        );
        assert_eq!(
            parse_rule_pick(r#"{"rule_index": 0, "confidence": 0.95}"#, 0),
            None
        );
    }

    #[test]
    fn rule_pick_rejects_malformed_or_missing_fields() {
        assert_eq!(parse_rule_pick("", 2), None);
        assert_eq!(parse_rule_pick("yes", 2), None);
        assert_eq!(parse_rule_pick("{not valid json}", 2), None);
        assert_eq!(parse_rule_pick(r#"{"rule_index": 0}"#, 2), None);
        assert_eq!(parse_rule_pick(r#"{"confidence": 0.95}"#, 2), None);
        assert_eq!(
            parse_rule_pick(r#"{"rule_index": "0", "confidence": 0.95}"#, 2),
            None
        );
    }
}
