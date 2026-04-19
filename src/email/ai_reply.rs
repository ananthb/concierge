//! AI-powered email reply generation using Workers AI

use serde::Serialize;
use worker::*;

const MODEL: &str = "@cf/meta/llama-3.1-8b-instruct";

#[derive(Serialize)]
struct AiRequest {
    messages: Vec<AiMessage>,
}

#[derive(Serialize)]
struct AiMessage {
    role: String,
    content: String,
}

const DEFAULT_SYSTEM_PROMPT: &str = "You are an email assistant. Draft a polite, professional reply to the email below. Keep it concise. Do not include subject line or headers, just the reply body.";

/// Generate an AI reply draft for an incoming email.
pub async fn generate_email_reply(
    env: &Env,
    system_prompt: Option<&str>,
    from: &str,
    subject: &str,
    body: &str,
) -> Result<String> {
    let prompt = system_prompt.unwrap_or(DEFAULT_SYSTEM_PROMPT);

    let user_message = format!("From: {from}\nSubject: {subject}\n\n{body}");

    let request = AiRequest {
        messages: vec![
            AiMessage {
                role: "system".into(),
                content: prompt.to_string(),
            },
            AiMessage {
                role: "user".into(),
                content: user_message,
            },
        ],
    };

    let request_json = serde_json::to_string(&request)
        .map_err(|e| Error::from(format!("Failed to serialize AI request: {e}")))?;

    let ai = env.ai("AI")?;

    let input: serde_json::Value = serde_json::from_str(&request_json)
        .map_err(|e| Error::from(format!("Failed to parse request: {e}")))?;

    let response: serde_json::Value = ai
        .run(MODEL, input)
        .await
        .map_err(|e| Error::from(format!("AI model error: {e:?}")))?;

    let reply = response
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| {
            response
                .get("response")
                .and_then(|r| r.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "Thank you for your email. I will get back to you shortly.".into());

    Ok(reply)
}
