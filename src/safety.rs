//! Content safety classifier using `@cf/meta/llama-guard-3-8b`.
//!
//! Invoked asynchronously from `safety_queue.rs` when a tenant saves their
//! business name or a canned reply message. Fails closed (`Rejected`) on
//! network errors or unparseable model output.

use serde::Serialize;
use worker::*;

const DEFAULT_SAFETY_MODEL: &str = "@cf/meta/llama-guard-3-8b";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyVerdict {
    Approved,
    Rejected { vague_reason: String },
}

#[derive(Serialize)]
struct GuardRequest {
    messages: Vec<GuardMessage>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct GuardMessage {
    role: String,
    content: String,
}

fn safety_model(env: &Env) -> String {
    env.var("AI_SAFETY_MODEL")
        .ok()
        .map(|v| v.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_SAFETY_MODEL.to_string())
}

/// Classify user-supplied business text (`brand_name` or canned reply) with
/// `@cf/meta/llama-guard-3-8b`. Fails closed (`Rejected`) on errors.
pub async fn classify_text(env: &Env, text: &str) -> SafetyVerdict {
    if crate::dev_bypass::active(env) {
        let _ = text;
        return SafetyVerdict::Approved;
    }

    let ai = match env.ai("AI") {
        Ok(a) => a,
        Err(_) => {
            return SafetyVerdict::Rejected {
                vague_reason: vague_reason_for("internal"),
            }
        }
    };

    let req = GuardRequest {
        messages: vec![GuardMessage {
            role: "user".to_string(),
            content: text.to_string(),
        }],
        max_tokens: 64,
    };

    let model = safety_model(env);
    let response: std::result::Result<serde_json::Value, _> = ai.run(&model, &req).await;
    let response = match response {
        Ok(r) => r,
        Err(e) => {
            console_log!("Llama Guard classifier error: {:?}", e);
            return SafetyVerdict::Rejected {
                vague_reason: vague_reason_for("internal"),
            };
        }
    };

    let raw = extract_response_text(&response).unwrap_or_default();
    parse_llama_guard_response(&raw)
}

fn extract_response_text(response: &serde_json::Value) -> Option<String> {
    response
        .as_str()
        .map(String::from)
        .or_else(|| {
            response
                .get("response")
                .and_then(|r| r.as_str())
                .map(String::from)
        })
        .or_else(|| {
            response
                .get("result")
                .and_then(|r| r.get("response"))
                .and_then(|r| r.as_str())
                .map(String::from)
        })
        .or_else(|| {
            response
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c0| c0.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .map(String::from)
        })
}

/// Parse Llama Guard 3 output (`"safe"` or `"unsafe\nS1"`). Fails closed on
/// anything unrecognized.
pub(crate) fn parse_llama_guard_response(raw: &str) -> SafetyVerdict {
    let mut lines = raw.trim().lines().map(str::trim).filter(|l| !l.is_empty());

    let Some(first) = lines.next() else {
        return SafetyVerdict::Rejected {
            vague_reason: vague_reason_for("internal"),
        };
    };

    if first.eq_ignore_ascii_case("safe") {
        return SafetyVerdict::Approved;
    }

    if first.eq_ignore_ascii_case("unsafe") {
        let category = lines
            .next()
            .and_then(|line| {
                line.split(|c: char| c == ',' || c.is_whitespace())
                    .find(|s| !s.is_empty())
            })
            .unwrap_or("policy");
        #[cfg(target_arch = "wasm32")]
        console_log!("Llama Guard rejected content: category={category}");
        return SafetyVerdict::Rejected {
            vague_reason: vague_reason_for(category),
        };
    }

    #[cfg(target_arch = "wasm32")]
    console_log!("Llama Guard returned unparseable verdict: {raw}");
    SafetyVerdict::Rejected {
        vague_reason: vague_reason_for("internal"),
    }
}

/// Map a Llama Guard 3 hazard category (`S1`–`S14`) to vague user-facing copy.
fn vague_reason_for(category: &str) -> String {
    match category.trim().to_ascii_uppercase().as_str() {
        "S1" | "S2" => {
            "This content doesn't fit our safety policies. Please remove any references to violence or harm."
                .to_string()
        }
        "S3" | "S4" | "S12" => {
            "This content doesn't fit our safety policies. Please keep text appropriate for a general audience."
                .to_string()
        }
        "S10" => {
            "This content doesn't fit our safety policies. Try softening language about specific groups of people."
                .to_string()
        }
        "S11" => {
            "This content doesn't fit our safety policies. Please remove any references to self-harm."
                .to_string()
        }
        "S5" | "S6" | "S7" | "S8" | "S9" | "S13" | "S14" => {
            "This content doesn't fit our safety policies. Please rewrite it for a standard business enquiry."
                .to_string()
        }
        _ => "This content doesn't fit our safety policies. Please rewrite it and try again."
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llama_guard_safe_approves() {
        assert_eq!(parse_llama_guard_response("safe"), SafetyVerdict::Approved);
        assert_eq!(
            parse_llama_guard_response("  SAFE \n"),
            SafetyVerdict::Approved
        );
    }

    #[test]
    fn llama_guard_unsafe_rejects_with_vague_reason() {
        match parse_llama_guard_response("unsafe\nS1") {
            SafetyVerdict::Rejected { vague_reason } => {
                assert!(!vague_reason.contains("S1"));
                assert!(vague_reason.contains("safety policies"));
            }
            SafetyVerdict::Approved => panic!("expected rejection"),
        }
        match parse_llama_guard_response("unsafe\nS10, S1") {
            SafetyVerdict::Rejected { vague_reason } => {
                assert!(!vague_reason.contains("S10"));
            }
            SafetyVerdict::Approved => panic!("expected rejection"),
        }
        assert!(matches!(
            parse_llama_guard_response("unsafe"),
            SafetyVerdict::Rejected { .. }
        ));
    }

    #[test]
    fn llama_guard_fails_closed_on_garbage() {
        assert!(matches!(
            parse_llama_guard_response(""),
            SafetyVerdict::Rejected { .. }
        ));
        assert!(matches!(
            parse_llama_guard_response("I think this is fine"),
            SafetyVerdict::Rejected { .. }
        ));
    }
}
