use serde::Serialize;
use worker::*;

const DEFAULT_MODEL: &str = "@cf/meta/llama-4-scout-17b-16e-instruct";
const DEFAULT_FAST_MODEL: &str = "@cf/meta/llama-3.1-8b-instruct-fast";
pub const EMBEDDING_MODEL: &str = "@cf/baai/bge-base-en-v1.5";

/// Hardcoded model for the demo persona business-detail generator.
/// Kimi K2.6's strength is structured JSON output, which is exactly
/// what that endpoint asks for. Not env-overrideable: the demo and
/// pipeline chat paths share the configurable `AI_MODEL` so the demo
/// faithfully represents production behavior; the persona generator is
/// the one place where a different model is justified.
const PERSONA_GEN_MODEL: &str = "@cf/moonshotai/kimi-k2.6";

fn get_model(env: &Env) -> String {
    env.var("AI_MODEL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| DEFAULT_MODEL.to_string())
}

fn get_fast_model(env: &Env) -> String {
    env.var("AI_FAST_MODEL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| DEFAULT_FAST_MODEL.to_string())
}

#[derive(Serialize)]
struct AiRequest {
    messages: Vec<Message>,
    /// Optional output cap. Only set for callers that need a long
    /// structured reply (e.g. the demo persona generator that returns
    /// a JSON array of N businesses). Skipped from the wire when None
    /// so existing chat callers continue to send the exact request
    /// shape they always have, dodging the "Workers AI rejects
    /// unrecognized request keys" footgun for model bindings that
    /// don't accept `max_tokens`.
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

// ============================================================================
// AI Response Generation
// ============================================================================

/// Generate an AI response using Cloudflare Workers AI
pub async fn generate_response(
    env: &Env,
    system_prompt: &str,
    fields_data: &serde_json::Map<String, serde_json::Value>,
) -> Result<String> {
    let form_context: String = fields_data
        .iter()
        .map(|(key, value)| {
            let val = match value {
                serde_json::Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            format!("{}: {}", key, val)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let user_message = format!(
        "Context:\n{}\n\nGenerate an appropriate response.",
        form_context
    );

    let request = AiRequest {
        messages: vec![
            Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: user_message,
            },
        ],
        max_tokens: None,
    };

    let model = get_model(env);
    run_ai_model(env, &model, &request).await
}

/// Generate a multi-turn chat reply. Distinct from `generate_response`,
/// which packs a context map into a single user message. Here, the
/// caller passes the actual `(role, content)` history and we forward
/// it verbatim. Used by the public `/demo/chat` endpoint AND the main
/// pipeline (which hands over the conversation history stored on the
/// `Session`).
///
/// Shape mirrors `generate_response` exactly (no extra request fields)
/// so it goes through the same Workers AI code path the lead form
/// already exercises in production. Caller is expected to keep replies
/// short via the system prompt; we don't pass `max_tokens` because
/// some Workers AI model bindings reject unrecognized request keys.
///
/// Empty history is allowed but unusual: the model will see only the
/// system prompt and have nothing to reply to. The pipeline always
/// appends the inbound that just arrived before calling, so this
/// degenerate case is reserved for tests / smoke checks.
pub async fn generate_chat_reply(
    env: &Env,
    system_prompt: &str,
    history: &[(String, String)],
) -> Result<String> {
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

/// Generate the demo persona JSON array via Kimi K2.6. Hardcoded
/// model: this is the one call where structured-output quality is
/// load-bearing and the cost (cache-absorbed) is small. `max_tokens`
/// is set high because the reply is a JSON array with one entry per
/// archetype; the chat model's default ~256-token cap truncates it
/// mid-string after three entries.
///
/// Sends the instructions and the archetype list as a single user
/// message rather than a system + user pair. Some Workers AI model
/// bindings (Kimi K2.6 in particular) ignore system messages and
/// fall through to a generic conversational reply (`"Thank you for
/// your message."`). Inlining keeps the contract robust across model
/// swaps.
pub async fn generate_persona_businesses(
    env: &Env,
    system_prompt: &str,
    user_prompt: &str,
    max_tokens: u32,
) -> Result<String> {
    let combined = format!("{system_prompt}\n\n{user_prompt}");
    let messages = vec![Message {
        role: "user".to_string(),
        content: combined,
    }];
    let request = AiRequest {
        messages,
        max_tokens: Some(max_tokens),
    };
    let reply = run_ai_model(env, PERSONA_GEN_MODEL, &request).await?;
    let trimmed = reply.trim();
    if trimmed.is_empty() {
        Err(Error::from("AI returned empty chat response"))
    } else {
        Ok(trimmed.to_string())
    }
}

// ============================================================================
// Prompt Injection Detection
// ============================================================================

/// Check if a message looks like a prompt injection attempt.
/// Returns true if injection is detected. Fails closed (returns true on error).
pub async fn is_prompt_injection(env: &Env, text: &str) -> bool {
    let model = get_fast_model(env);
    // Skip very short messages
    if text.len() < 10 {
        return false;
    }

    let request = AiRequest {
        messages: vec![
            Message {
                role: "system".to_string(),
                content: crate::prompt::INJECTION_SCANNER.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: text.to_string(),
            },
        ],
        max_tokens: None,
    };

    let ai = match env.ai("AI") {
        Ok(a) => a,
        Err(_) => return true, // fail closed
    };

    // Pass the struct directly. See `run_ai_model` for the JS Map vs Object trap.
    let result: std::result::Result<serde_json::Value, _> = ai.run(&model, &request).await;
    match result {
        Ok(response) => {
            let answer = response
                .as_str()
                .or_else(|| {
                    response
                        .get("response")
                        .and_then(|r: &serde_json::Value| r.as_str())
                })
                .unwrap_or("YES");
            answer.trim().to_uppercase().starts_with("YES")
        }
        Err(e) => {
            console_log!("Injection scanner error: {:?}", e);
            true // fail closed
        }
    }
}

// ============================================================================
// Embeddings (rule matching)
// ============================================================================

/// Embed a single piece of text using Cloudflare Workers AI's BGE model.
/// Returns the dense vector. Used by the pipeline (embed inbound message)
/// and by the persona admin handler (embed each Prompt rule's description
/// on save).
pub async fn embed(env: &Env, text: &str) -> Result<Vec<f32>> {
    #[derive(Serialize)]
    struct EmbedRequest<'a> {
        text: [&'a str; 1],
    }
    let ai = env.ai("AI")?;
    // Pass the struct directly. See `run_ai_model` for the JS Map vs Object trap.
    let response: serde_json::Value = ai
        .run(EMBEDDING_MODEL, &EmbedRequest { text: [text] })
        .await
        .map_err(|e| Error::from(format!("Embedding model error: {:?}", e)))?;

    // BGE returns { "data": [[..floats..]], "shape": [...] }. Defensively
    // accept either `data` or `embeddings`, and either nested-list or flat
    // single-vector layouts.
    let arr = response
        .get("data")
        .or_else(|| response.get("embeddings"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::from("Embedding response missing data array"))?;
    let first = arr
        .first()
        .ok_or_else(|| Error::from("Embedding response data array empty"))?;
    let vec = first
        .as_array()
        .ok_or_else(|| Error::from("Embedding response inner not array"))?;
    Ok(vec
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect())
}

/// Cosine similarity in [-1.0, 1.0]. Returns 0 on length mismatch or
/// zero-magnitude vectors so callers don't need to special-case those.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

// ============================================================================
// Internal
// ============================================================================

async fn run_ai_model(env: &Env, model: &str, request: &AiRequest) -> Result<String> {
    let ai = env.ai("AI")?;

    // CRITICAL: pass the `AiRequest` struct directly to `ai.run`. Round-tripping
    // through `serde_json::Value` and then handing the resulting Value to the
    // worker-rs binding makes `serde_wasm_bindgen` serialize the inner
    // `Map<String, Value>` as a JavaScript `Map` (the ES6 `new Map(...)` type)
    // rather than a plain Object. Workers AI's input schema validates against a
    // plain Object, sees no own properties on the Map, and returns:
    //   AiError 5006: oneOf at '/' not met, 0 matches: required properties at
    //   '/' are 'prompt', 'messages', 'requests'.
    // Serializing the struct directly emits a real Object via `serialize_struct`.
    let response: serde_json::Value = ai
        .run(model, request)
        .await
        .map_err(|e| Error::from(format!("AI model error: {:?}", e)))?;

    let response_str = response
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| {
            response
                .get("response")
                .and_then(|r| r.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "Thank you for your message.".to_string());

    Ok(response_str)
}
