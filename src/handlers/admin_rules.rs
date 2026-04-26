//! Unified per-channel reply-rules CRUD: `/admin/rules/{channel}/{id}/...`.
//!
//! All four conversational channels (WhatsApp, Instagram, Email, Discord)
//! store a `ReplyConfig` with the same shape, so the CRUD logic lives once
//! here and dispatches to the right loader/saver via `ChannelRef`.
//!
//! Routes:
//!   GET    /admin/rules/{ch}/{id}                  list page
//!   GET    /admin/rules/{ch}/{id}/new              new-rule form
//!   POST   /admin/rules/{ch}/{id}                  create rule
//!   GET    /admin/rules/{ch}/{id}/{rule_id}        edit-rule form
//!   PUT    /admin/rules/{ch}/{id}/{rule_id}        update rule
//!   DELETE /admin/rules/{ch}/{id}/{rule_id}        delete rule
//!   POST   /admin/rules/{ch}/{id}/{rule_id}/move/up
//!   POST   /admin/rules/{ch}/{id}/{rule_id}/move/down
//!   GET    /admin/rules/{ch}/{id}/default          edit-default form
//!   PUT    /admin/rules/{ch}/{id}/default          update default rule
//!
//! `ch` is one of: `whatsapp`, `instagram`, `email`, `discord`.
//! `id` for `discord` is the literal string `_` (single config per tenant).

use worker::*;

use crate::ai;
use crate::helpers::{generate_id, now_iso};
use crate::storage::*;
use crate::templates::rules::{rule_form_html, rule_form_title, rules_list_html};
use crate::types::{default_match_threshold, ReplyConfig, ReplyMatcher, ReplyResponse, ReplyRule};

const MAX_LABEL: usize = 80;
const MAX_KEYWORDS: usize = 20;
const MAX_KEYWORD_LEN: usize = 80;
const MAX_DESCRIPTION: usize = 200;
const MAX_RESPONSE: usize = 2000;
const MAX_RULES: usize = 50;

pub enum ChannelRef<'a> {
    WhatsApp { id: &'a str },
    Instagram { id: &'a str },
    Email { label: &'a str },
    Discord,
}

impl<'a> ChannelRef<'a> {
    fn slug(&self) -> &'static str {
        match self {
            ChannelRef::WhatsApp { .. } => "whatsapp",
            ChannelRef::Instagram { .. } => "instagram",
            ChannelRef::Email { .. } => "email",
            ChannelRef::Discord => "discord",
        }
    }

    fn id_part(&self) -> &str {
        match self {
            ChannelRef::WhatsApp { id } => id,
            ChannelRef::Instagram { id } => id,
            ChannelRef::Email { label } => label,
            ChannelRef::Discord => "_",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ChannelRef::WhatsApp { .. } => "WhatsApp",
            ChannelRef::Instagram { .. } => "Instagram",
            ChannelRef::Email { .. } => "Email",
            ChannelRef::Discord => "Discord",
        }
    }

    pub fn back_url(&self, base: &str) -> String {
        match self {
            ChannelRef::WhatsApp { id } => format!("{base}/admin/whatsapp/{id}"),
            ChannelRef::Instagram { id } => format!("{base}/admin/instagram/{id}"),
            ChannelRef::Email { label } => format!("{base}/admin/email/addresses/{label}"),
            ChannelRef::Discord => format!("{base}/admin/discord"),
        }
    }

    pub fn rules_base(&self, base: &str) -> String {
        format!("{base}/admin/rules/{}/{}", self.slug(), self.id_part())
    }

    async fn load(&self, kv: &kv::KvStore, tenant_id: &str) -> Result<Option<ReplyConfig>> {
        match self {
            ChannelRef::WhatsApp { id } => Ok(get_whatsapp_account(kv, id)
                .await?
                .filter(|a| a.tenant_id == tenant_id)
                .map(|a| a.auto_reply)),
            ChannelRef::Instagram { id } => Ok(get_instagram_account(kv, id)
                .await?
                .filter(|a| a.tenant_id == tenant_id)
                .map(|a| a.auto_reply)),
            ChannelRef::Email { label } => Ok(get_email_address(kv, tenant_id, label)
                .await?
                .map(|a| a.auto_reply)),
            ChannelRef::Discord => Ok(get_discord_config_by_tenant(kv, tenant_id)
                .await?
                .map(|c| c.auto_reply)),
        }
    }

    async fn save(&self, kv: &kv::KvStore, tenant_id: &str, cfg: ReplyConfig) -> Result<bool> {
        let now = now_iso();
        match self {
            ChannelRef::WhatsApp { id } => {
                let Some(mut account) = get_whatsapp_account(kv, id).await? else {
                    return Ok(false);
                };
                if account.tenant_id != tenant_id {
                    return Ok(false);
                }
                account.auto_reply = cfg;
                account.updated_at = now;
                save_whatsapp_account(kv, &account).await?;
                Ok(true)
            }
            ChannelRef::Instagram { id } => {
                let Some(mut account) = get_instagram_account(kv, id).await? else {
                    return Ok(false);
                };
                if account.tenant_id != tenant_id {
                    return Ok(false);
                }
                account.auto_reply = cfg;
                account.updated_at = now;
                save_instagram_account(kv, &account).await?;
                Ok(true)
            }
            ChannelRef::Email { label } => {
                let Some(mut addr) = get_email_address(kv, tenant_id, label).await? else {
                    return Ok(false);
                };
                addr.auto_reply = cfg;
                addr.updated_at = now;
                save_email_address(kv, tenant_id, &addr).await?;
                Ok(true)
            }
            ChannelRef::Discord => {
                let Some(mut dc) = get_discord_config_by_tenant(kv, tenant_id).await? else {
                    return Ok(false);
                };
                dc.auto_reply = cfg;
                save_discord_config(kv, &dc).await?;
                Ok(true)
            }
        }
    }
}

/// Parse a `/admin/rules/{ch}/{id}/...` path into a `ChannelRef` plus the
/// remaining sub-path segments.
fn parse_path<'a>(path: &'a str) -> Option<(ChannelRef<'a>, Vec<&'a str>)> {
    let parts: Vec<&str> = path
        .strip_prefix("/admin/rules/")?
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() < 2 {
        return None;
    }
    let channel = match parts[0] {
        "whatsapp" => ChannelRef::WhatsApp { id: parts[1] },
        "instagram" => ChannelRef::Instagram { id: parts[1] },
        "email" => ChannelRef::Email { label: parts[1] },
        "discord" => ChannelRef::Discord,
        _ => return None,
    };
    Some((channel, parts.into_iter().skip(2).collect()))
}

pub async fn handle_rules(
    mut req: Request,
    env: Env,
    path: &str,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let kv = env.kv("KV")?;
    let method = req.method();

    let Some((channel, rest)) = parse_path(path) else {
        return Response::error("Not Found", 404);
    };

    let Some(mut cfg) = channel.load(&kv, tenant_id).await? else {
        return Response::error("Channel not found", 404);
    };

    let rest_slice: Vec<&str> = rest.iter().copied().collect();
    match (method, rest_slice.as_slice()) {
        // List page
        (Method::Get, []) => Response::from_html(rules_list_html(&cfg, &channel, base_url)),

        // New-rule form
        (Method::Get, ["new"]) => {
            Response::from_html(rule_form_html(&channel, None, base_url, "Add a rule"))
        }

        // Create rule
        (Method::Post, []) => {
            if cfg.rules.len() >= MAX_RULES {
                return Response::from_html(format!(
                    r#"<div class="error">You've reached the rule cap ({MAX_RULES}). Delete one before adding another.</div>"#
                ));
            }
            let form: serde_json::Value = req.json().await?;
            let rule = match build_rule_from_form(&env, &generate_id(), &form).await {
                Ok(r) => r,
                Err(msg) => {
                    return Response::from_html(format!(r#"<div class="error">{msg}</div>"#));
                }
            };
            cfg.rules.push(rule);
            channel.save(&kv, tenant_id, cfg).await?;
            redirect_to(base_url, &channel)
        }

        // Edit default rule
        (Method::Get, ["default"]) => Response::from_html(rule_form_html(
            &channel,
            Some(&cfg.default_rule),
            base_url,
            "Edit default reply",
        )),

        // Update default rule
        (Method::Put, ["default"]) => {
            let form: serde_json::Value = req.json().await?;
            // The default rule keeps `Default` matcher and id="default" — the
            // form only edits the response.
            let mode = form
                .get("response_kind")
                .and_then(|v| v.as_str())
                .unwrap_or("prompt");
            let text: String = form
                .get("response_text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .chars()
                .take(MAX_RESPONSE)
                .collect();
            cfg.default_rule.response = match mode {
                "canned" => ReplyResponse::Canned { text },
                _ => ReplyResponse::Prompt { text },
            };
            // Allow renaming the default rule's label so admins can describe
            // their fallback ("General fallback", "After-hours", etc.).
            if let Some(label) = form.get("label").and_then(|v| v.as_str()) {
                let trimmed: String = label.trim().chars().take(MAX_LABEL).collect();
                if !trimmed.is_empty() {
                    cfg.default_rule.label = trimmed;
                }
            }
            channel.save(&kv, tenant_id, cfg).await?;
            redirect_to(base_url, &channel)
        }

        // Edit-rule form
        (Method::Get, [rule_id]) if *rule_id != "new" => {
            let Some(existing) = cfg.rules.iter().find(|r| r.id == *rule_id).cloned() else {
                return Response::error("Rule not found", 404);
            };
            Response::from_html(rule_form_html(
                &channel,
                Some(&existing),
                base_url,
                rule_form_title(&existing),
            ))
        }

        // Update rule
        (Method::Put, [rule_id]) => {
            let id = (*rule_id).to_string();
            let Some(idx) = cfg.rules.iter().position(|r| r.id == id) else {
                return Response::error("Rule not found", 404);
            };
            let form: serde_json::Value = req.json().await?;
            let updated = match build_rule_from_form(&env, &id, &form).await {
                Ok(r) => r,
                Err(msg) => {
                    return Response::from_html(format!(r#"<div class="error">{msg}</div>"#));
                }
            };
            cfg.rules[idx] = updated;
            channel.save(&kv, tenant_id, cfg).await?;
            redirect_to(base_url, &channel)
        }

        // Delete rule
        (Method::Delete, [rule_id]) => {
            let id = (*rule_id).to_string();
            let before = cfg.rules.len();
            cfg.rules.retain(|r| r.id != id);
            if cfg.rules.len() == before {
                return Response::error("Rule not found", 404);
            }
            channel.save(&kv, tenant_id, cfg).await?;
            // HTMX delete swaps the row out via hx-target on the row itself.
            Response::ok("")
        }

        // Reorder rule
        (Method::Post, [rule_id, "move", direction]) => {
            let id = (*rule_id).to_string();
            let Some(idx) = cfg.rules.iter().position(|r| r.id == id) else {
                return Response::error("Rule not found", 404);
            };
            let new_idx = match *direction {
                "up" if idx > 0 => idx - 1,
                "down" if idx + 1 < cfg.rules.len() => idx + 1,
                _ => idx,
            };
            if new_idx != idx {
                cfg.rules.swap(idx, new_idx);
                channel.save(&kv, tenant_id, cfg).await?;
            }
            redirect_to(base_url, &channel)
        }

        _ => Response::error("Not Found", 404),
    }
}

fn redirect_to(base_url: &str, channel: &ChannelRef<'_>) -> Result<Response> {
    let target = channel.rules_base(base_url);
    let headers = Headers::new();
    headers.set("HX-Redirect", &target)?;
    headers.set("Location", &target)?;
    Ok(Response::empty()?.with_status(200).with_headers(headers))
}

/// Parse an add/edit form into a `ReplyRule`, embedding Prompt rule
/// descriptions through Workers AI. Returns a user-facing error string
/// on validation/embedding failure.
async fn build_rule_from_form(
    env: &Env,
    id: &str,
    form: &serde_json::Value,
) -> std::result::Result<ReplyRule, String> {
    let label = form
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .chars()
        .take(MAX_LABEL)
        .collect::<String>();
    if label.is_empty() {
        return Err("Give the rule a short label.".to_string());
    }

    let matcher_kind = form
        .get("matcher_kind")
        .and_then(|v| v.as_str())
        .unwrap_or("static_text");

    let matcher = match matcher_kind {
        "static_text" => {
            let raw = form.get("keywords").and_then(|v| v.as_str()).unwrap_or("");
            let keywords: Vec<String> = raw
                .split(|c: char| c == ',' || c == '\n')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .map(|s| s.chars().take(MAX_KEYWORD_LEN).collect())
                .take(MAX_KEYWORDS)
                .collect();
            if keywords.is_empty() {
                return Err("Add at least one keyword (comma- or newline-separated).".to_string());
            }
            ReplyMatcher::StaticText { keywords }
        }
        "prompt" => {
            let description: String = form
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .chars()
                .take(MAX_DESCRIPTION)
                .collect();
            if description.is_empty() {
                return Err(
                    "Describe what kind of message should match (e.g. 'asks about hours').".into(),
                );
            }
            let threshold = form
                .get("threshold")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32)
                .unwrap_or_else(default_match_threshold)
                .clamp(0.5, 0.95);

            // Embed synchronously on save. If the AI binding is down, refuse
            // the save: a Prompt rule with no embedding can never match,
            // which would silently degrade routing.
            let embedding = ai::embed(env, &description)
                .await
                .map_err(|e| format!("Embedding failed: {e}. Try again in a moment."))?;
            if embedding.is_empty() {
                return Err("Embedding came back empty. Try again.".to_string());
            }
            ReplyMatcher::Prompt {
                description,
                embedding,
                embedding_model: ai::EMBEDDING_MODEL.to_string(),
                threshold,
            }
        }
        _ => return Err("Pick a matcher type.".to_string()),
    };

    let response_kind = form
        .get("response_kind")
        .and_then(|v| v.as_str())
        .unwrap_or("canned");
    let response_text: String = form
        .get("response_text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .chars()
        .take(MAX_RESPONSE)
        .collect();
    if response_text.trim().is_empty() {
        return Err("Write the reply text or AI prompt.".to_string());
    }
    let response = match response_kind {
        "prompt" => ReplyResponse::Prompt {
            text: response_text,
        },
        _ => ReplyResponse::Canned {
            text: response_text,
        },
    };

    Ok(ReplyRule {
        id: id.to_string(),
        label,
        matcher,
        response,
    })
}
