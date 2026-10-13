//! Wire a tenant's domain into Cloudflare Email Routing.
//!
//! When a tenant connects a domain, this points its catch-all at this Worker
//! so inbound mail reaches the `email` handler, enabling Email Routing first
//! (which creates the MX/SPF/DKIM records) if it isn't already on.
//!
//! Everything here runs against the Worker's own scoped token via
//! [`crate::cloudflare::request`]. The domain must be a zone in the same
//! Cloudflare account: Email Routing's "send to Worker" action only targets a
//! Worker in the account, so a domain hosted elsewhere can't be wired this way.
//!
//! Outbound (Email Sending domain onboarding) is not here yet; replies use the
//! `send_email` binding and require the sending domain to be onboarded
//! separately.

use worker::*;

use crate::cloudflare;

/// The Worker name Email Routing delivers to. Matches `wrangler.toml`'s
/// `name` and the platform's `cloudflare_workers_custom_domain` service.
pub const WORKER_NAME: &str = "concierge";

/// Point `domain`'s catch-all at this Worker, enabling Email Routing if it
/// isn't already. Idempotent: re-running on a wired domain is a no-op.
pub async fn enable_inbound(env: &Env, domain: &str) -> Result<()> {
    let zone_id = zone_id_for(env, domain).await?;

    // `POST .../email/routing/dns` enables routing and creates the records;
    // skip it when routing is already on.
    let settings = cloudflare::request(
        env,
        Method::Get,
        &format!("/zones/{zone_id}/email/routing"),
        None,
    )
    .await?;
    let already_enabled = settings
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !already_enabled {
        cloudflare::request(
            env,
            Method::Post,
            &format!("/zones/{zone_id}/email/routing/dns"),
            None,
        )
        .await?;
    }

    cloudflare::request(
        env,
        Method::Put,
        &format!("/zones/{zone_id}/email/routing/rules/catch_all"),
        Some(catch_all_body(WORKER_NAME)),
    )
    .await?;

    Ok(())
}

/// Resolve a domain to its zone id, or fail if it isn't in the account.
async fn zone_id_for(env: &Env, domain: &str) -> Result<String> {
    let result =
        cloudflare::request(env, Method::Get, &format!("/zones?name={domain}"), None).await?;
    zone_id_from(&result)
        .ok_or_else(|| Error::from(format!("{domain} is not a zone in this Cloudflare account")))
}

/// The first zone id in a `GET /zones?name=` result. Pure, for tests.
pub fn zone_id_from(result: &serde_json::Value) -> Option<String> {
    result
        .as_array()?
        .first()?
        .get("id")?
        .as_str()
        .map(str::to_string)
}

/// The catch-all rule body: match everything, deliver to `worker`. Pure, for
/// tests.
pub fn catch_all_body(worker: &str) -> serde_json::Value {
    serde_json::json!({
        "name": "concierge",
        "enabled": true,
        "matchers": [{ "type": "all" }],
        "actions": [{ "type": "worker", "value": [worker] }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catch_all_targets_the_worker() {
        let body = catch_all_body("concierge");
        assert_eq!(body["matchers"][0]["type"], "all");
        assert_eq!(body["actions"][0]["type"], "worker");
        assert_eq!(body["actions"][0]["value"][0], "concierge");
        assert_eq!(body["enabled"], true);
    }

    #[test]
    fn zone_id_from_picks_the_first_zone() {
        let result = serde_json::json!([
            { "id": "abc123", "name": "acme.com" },
            { "id": "def456", "name": "other.com" }
        ]);
        assert_eq!(zone_id_from(&result).as_deref(), Some("abc123"));
    }

    #[test]
    fn zone_id_from_none_when_absent() {
        assert_eq!(zone_id_from(&serde_json::json!([])), None);
        assert_eq!(zone_id_from(&serde_json::Value::Null), None);
    }
}
