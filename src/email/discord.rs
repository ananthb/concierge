//! Discord bot integration for email forwarding

use serde::Serialize;
use worker::*;

const DISCORD_API_BASE: &str = "https://discord.com/api/v10";

#[derive(Serialize)]
struct CreateMessage {
    content: Option<String>,
    embeds: Option<Vec<Embed>>,
}

#[derive(Serialize)]
struct Embed {
    title: Option<String>,
    description: Option<String>,
    color: Option<u32>,
    fields: Vec<EmbedField>,
    footer: Option<EmbedFooter>,
}

#[derive(Serialize)]
struct EmbedField {
    name: String,
    value: String,
    inline: bool,
}

#[derive(Serialize)]
struct EmbedFooter {
    text: String,
}

/// Post an email summary to a Discord channel as an embed.
pub async fn post_email_to_discord(
    bot_token: &str,
    channel_id: &str,
    from: &str,
    to: &str,
    subject: &str,
    body_preview: &str,
    rule_name: &str,
) -> Result<()> {
    let body_truncated = if body_preview.len() > 1000 {
        format!("{}...", &body_preview[..997])
    } else {
        body_preview.to_string()
    };

    let message = CreateMessage {
        content: None,
        embeds: Some(vec![Embed {
            title: Some(if subject.is_empty() {
                "(no subject)".into()
            } else {
                subject.to_string()
            }),
            description: Some(body_truncated),
            color: Some(0xF38020), // Cloudflare orange
            fields: vec![
                EmbedField {
                    name: "From".into(),
                    value: from.to_string(),
                    inline: true,
                },
                EmbedField {
                    name: "To".into(),
                    value: to.to_string(),
                    inline: true,
                },
            ],
            footer: Some(EmbedFooter {
                text: format!("Rule: {rule_name}"),
            }),
        }]),
    };

    let url = format!("{DISCORD_API_BASE}/channels/{channel_id}/messages");
    let body =
        serde_json::to_string(&message).map_err(|e| Error::from(format!("JSON error: {e}")))?;

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bot {bot_token}"))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(&body)));

    let request = Request::new_with_init(&url, &init)?;
    let resp = Fetch::Request(request).send().await?;

    if resp.status_code() >= 400 {
        console_log!(
            "Discord API error: status={} channel={}",
            resp.status_code(),
            channel_id
        );
    }

    Ok(())
}

/// Post an AI draft reply to Discord for approval (with reaction prompt).
pub async fn post_ai_draft_for_approval(
    bot_token: &str,
    channel_id: &str,
    from: &str,
    to: &str,
    subject: &str,
    original_body: &str,
    ai_draft: &str,
    rule_name: &str,
) -> Result<()> {
    let original_truncated = if original_body.len() > 500 {
        format!("{}...", &original_body[..497])
    } else {
        original_body.to_string()
    };

    let message = CreateMessage {
        content: Some("**AI Draft Reply**: React with ✅ to approve, ❌ to discard.".to_string()),
        embeds: Some(vec![
            Embed {
                title: Some(format!("Re: {subject}")),
                description: Some(format!("**Draft reply:**\n{ai_draft}")),
                color: Some(0x5865F2), // Discord blurple
                fields: vec![
                    EmbedField {
                        name: "From".into(),
                        value: from.to_string(),
                        inline: true,
                    },
                    EmbedField {
                        name: "To".into(),
                        value: to.to_string(),
                        inline: true,
                    },
                ],
                footer: Some(EmbedFooter {
                    text: format!("Rule: {rule_name}"),
                }),
            },
            Embed {
                title: Some("Original message".into()),
                description: Some(original_truncated),
                color: Some(0x99AAB5),
                fields: vec![],
                footer: None,
            },
        ]),
    };

    let url = format!("{DISCORD_API_BASE}/channels/{channel_id}/messages");
    let body =
        serde_json::to_string(&message).map_err(|e| Error::from(format!("JSON error: {e}")))?;

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bot {bot_token}"))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(&body)));

    let request = Request::new_with_init(&url, &init)?;
    let resp = Fetch::Request(request).send().await?;

    if resp.status_code() >= 400 {
        console_log!(
            "Discord API error (AI draft): status={} channel={}",
            resp.status_code(),
            channel_id
        );
    }

    Ok(())
}
