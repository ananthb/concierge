//! `/api/email/*` — the tenant's connected custom email domains.
//!
//! A tenant adds a domain here; inbound mail for it reaches the Worker's
//! `email` event (see `email::inbound`) and is answered by the shared
//! pipeline. The DNS side — enabling Cloudflare Email Routing, or adding
//! the MX/SPF records by hand — is the tenant's, described by the `setup`
//! object so the frontend can show the exact records.

use serde::Serialize;
use worker::*;

use crate::helpers::{generate_id, now_iso, truncate};
use crate::storage::*;
use crate::types::*;

#[derive(Serialize)]
pub struct AccountView {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub local_part: String,
    pub from_name: String,
    /// Full address replies are sent from, e.g. "hello@acme.com".
    pub from_address: String,
    pub reply: ReplyView,
    pub created_at: String,
}

/// The auto-reply settings for one address. Same shape as the WhatsApp
/// view so the frontend's editor is shared.
#[derive(Serialize)]
pub struct ReplyView {
    pub enabled: bool,
    pub mode: &'static str,
    pub text: String,
    pub wait_seconds: u32,
}

/// DNS the tenant must have in place for inbound mail to arrive and
/// outbound mail to authenticate. Cloudflare Email Routing creates all of
/// these when enabled on a zone it manages; the fields are here for
/// domains pointed at another provider.
#[derive(Serialize)]
pub struct SetupView {
    pub mx_hosts: [&'static str; 3],
    pub spf: &'static str,
    pub note: &'static str,
}

impl SetupView {
    fn cloudflare() -> Self {
        Self {
            mx_hosts: [
                "route1.mx.cloudflare.net",
                "route2.mx.cloudflare.net",
                "route3.mx.cloudflare.net",
            ],
            spf: "v=spf1 include:_spf.mx.cloudflare.net ~all",
            note: "Enable Cloudflare Email Routing on the domain's zone; it creates the MX, \
                   SPF, DKIM and DMARC records for you. If the domain uses another DNS \
                   provider, add the MX hosts and SPF record above.",
        }
    }
}

#[derive(Serialize)]
struct ListView {
    accounts: Vec<AccountView>,
    setup: SetupView,
}

impl From<&EmailAccount> for AccountView {
    fn from(a: &EmailAccount) -> Self {
        let (mode, text) = match &a.auto_reply.response {
            ReplyResponse::Canned { text } => ("canned", text.clone()),
            ReplyResponse::Prompt { text } => ("prompt", text.clone()),
        };
        AccountView {
            id: a.id.clone(),
            name: a.name.clone(),
            domain: a.domain.clone(),
            local_part: a.local_part.clone(),
            from_name: a.from_name.clone(),
            from_address: a.from_address(),
            reply: ReplyView {
                enabled: a.auto_reply.enabled,
                mode,
                text,
                wait_seconds: a.auto_reply.wait_seconds,
            },
            created_at: a.created_at.clone(),
        }
    }
}

pub async fn handle(mut req: Request, env: Env, rest: &[&str]) -> Result<Response> {
    let tenant_id = match super::require_tenant(&req, &env).await {
        Ok(t) => t,
        Err(r) => return r,
    };
    let kv = env.kv("KV")?;
    let method = req.method();

    match (method, rest) {
        (Method::Get, []) => {
            let accounts = list_email_accounts(&kv, &tenant_id).await?;
            super::json(&ListView {
                accounts: accounts.iter().map(AccountView::from).collect(),
                setup: SetupView::cloudflare(),
            })
        }

        (Method::Post, []) => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            connect(&env, &kv, &tenant_id, &body).await
        }

        (Method::Put, [id]) => {
            let Some(mut account) = owned(&kv, &tenant_id, id).await? else {
                return super::not_found("Domain");
            };
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };

            if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
                account.name = truncate(name.trim(), 200);
            }
            if let Some(from_name) = body.get("from_name").and_then(|v| v.as_str()) {
                account.from_name = truncate(from_name.trim(), 200);
            }
            if let Some(reply) = body.get("reply") {
                account.auto_reply.enabled = super::field_bool(reply, "enabled");
                let mode = super::field(reply, "mode");
                let text = truncate(&super::field(reply, "text"), 2000);
                if mode == "prompt" && text.is_empty() {
                    return super::invalid("Write the instruction the AI should follow.");
                }
                if mode == "canned" && text.is_empty() && account.auto_reply.enabled {
                    return super::invalid("Write the message to send.");
                }
                account.auto_reply.set_default_response(&mode, text);
                if let Some(n) = super::field_i64(reply, "wait_seconds") {
                    account.auto_reply.wait_seconds = n.clamp(0, 30) as u32;
                }
            }

            account.updated_at = now_iso();
            save_email_account(&kv, &account).await?;
            super::json(&AccountView::from(&account))
        }

        (Method::Delete, [id]) => {
            if owned(&kv, &tenant_id, id).await?.is_none() {
                return super::not_found("Domain");
            }
            delete_email_account(&kv, &tenant_id, id).await?;
            super::no_content()
        }

        _ => super::not_found("Endpoint"),
    }
}

async fn connect(
    env: &Env,
    kv: &kv::KvStore,
    tenant_id: &str,
    body: &serde_json::Value,
) -> Result<Response> {
    let domain = normalize_domain(&super::field(body, "domain"));
    if !is_valid_domain(&domain) {
        return super::invalid("Enter a domain like acme.com.");
    }
    let local_part = normalize_local_part(&super::field(body, "local_part"));
    if !local_part.is_empty() && !is_valid_local_part(&local_part) {
        return super::invalid(
            "Use letters, numbers, dots, hyphens or underscores for the address.",
        );
    }

    // One tenant per domain: another tenant's catch-all would be shadowed.
    if let Some(owner) = email_domain_owner(kv, &domain).await? {
        if owner != tenant_id {
            return super::invalid("That domain is connected to another account.");
        }
    }
    for existing in list_email_accounts(kv, tenant_id).await? {
        if existing.domain == domain && existing.local_part == local_part {
            return super::invalid("That address is already connected.");
        }
    }

    let onboarding = get_onboarding(kv, tenant_id).await.unwrap_or_default();
    let now = now_iso();
    let account = EmailAccount {
        id: generate_id(),
        tenant_id: tenant_id.to_string(),
        name: truncate(&super::field(body, "name"), 200),
        domain,
        local_part,
        from_name: truncate(&super::field(body, "from_name"), 200),
        auto_reply: ReplyConfig {
            enabled: false,
            response: crate::types::default_response(),
            wait_seconds: onboarding.default_wait_seconds,
        },
        created_at: now.clone(),
        updated_at: now,
    };
    save_email_account(kv, &account).await?;

    // Best-effort wiring: point the zone's Email Routing catch-all at this
    // Worker. A failure (the domain isn't a zone in this account, or the token
    // is missing) is logged and does not block the connection -- the domain is
    // recorded regardless, and the dashboard still shows the DNS to add.
    if let Err(e) = crate::email::onboarding::enable_inbound(env, &account.domain).await {
        worker::console_log!("Email onboarding failed for {}: {:?}", account.domain, e);
    }

    super::json(&AccountView::from(&account))
}

/// Load an account only if this tenant owns it (see the WhatsApp module for
/// why ownership is checked here rather than by the caller).
async fn owned(kv: &kv::KvStore, tenant_id: &str, id: &str) -> Result<Option<EmailAccount>> {
    Ok(get_email_account(kv, id)
        .await?
        .filter(|a| a.tenant_id == tenant_id))
}

fn normalize_domain(s: &str) -> String {
    s.trim().trim_end_matches('.').to_ascii_lowercase()
}

fn normalize_local_part(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

/// A pragmatic domain check: at least one dot, RFC-length labels, no
/// leading/trailing hyphen. Deliberately not a full IDNA/exotic-TLD
/// validator — the value is also constrained by DNS actually resolving.
fn is_valid_domain(domain: &str) -> bool {
    if domain.len() < 4 || domain.len() > 253 || !domain.contains('.') {
        return false;
    }
    domain.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}

fn is_valid_local_part(local: &str) -> bool {
    !local.is_empty()
        && local.len() <= 64
        && local
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_domains_pass() {
        assert!(is_valid_domain("acme.com"));
        assert!(is_valid_domain("sub.acme.co.uk"));
        assert!(is_valid_domain("x1-y.example.io"));
    }

    #[test]
    fn invalid_domains_fail() {
        assert!(!is_valid_domain("acme"));
        assert!(!is_valid_domain("acme."));
        assert!(!is_valid_domain(".acme.com"));
        assert!(!is_valid_domain("-acme.com"));
        assert!(!is_valid_domain("acme..com"));
        assert!(!is_valid_domain("ac me.com"));
    }

    #[test]
    fn normalize_domain_lowercases_and_drops_trailing_dot() {
        assert_eq!(normalize_domain("  Acme.COM. "), "acme.com");
    }

    #[test]
    fn local_part_rules() {
        assert!(is_valid_local_part("hello"));
        assert!(is_valid_local_part("sales.team"));
        assert!(is_valid_local_part("hello+tags"));
        assert!(!is_valid_local_part(""));
        assert!(!is_valid_local_part("he llo"));
        assert!(!is_valid_local_part("a@b"));
    }
}
