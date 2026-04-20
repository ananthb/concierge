//! Cloudflare DNS API client for email subdomain MX record provisioning.

use serde::Deserialize;
use worker::*;

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";

#[derive(Deserialize)]
struct CfApiResponse {
    success: bool,
    #[serde(default)]
    errors: Vec<CfApiError>,
    result: Option<CfDnsRecord>,
}

#[derive(Deserialize)]
struct CfApiError {
    message: String,
}

#[derive(Deserialize)]
struct CfDnsRecord {
    id: String,
}

/// Validate a subdomain label (the part before the base domain).
pub fn validate_subdomain_label(label: &str) -> Result<(), &'static str> {
    if label.len() < 3 || label.len() > 63 {
        return Err("Must be 3-63 characters");
    }
    if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("Only letters, numbers, and hyphens allowed");
    }
    if label.starts_with('-') || label.ends_with('-') {
        return Err("Cannot start or end with a hyphen");
    }
    if label.contains("--") {
        return Err("Cannot contain consecutive hyphens");
    }

    const RESERVED: &[&str] = &[
        "www",
        "mail",
        "admin",
        "api",
        "ns1",
        "ns2",
        "smtp",
        "imap",
        "pop",
        "ftp",
        "_dmarc",
        "autoconfig",
        "autodiscover",
    ];
    if RESERVED.contains(&label) {
        return Err("This name is reserved");
    }

    Ok(())
}

/// Create MX records for a subdomain on the Cloudflare zone.
/// Returns the DNS record IDs on success.
pub async fn create_mx_records(
    zone_id: &str,
    subdomain_label: &str,
    base_domain: &str,
    mx_primary: &str,
    mx_secondary: &str,
    api_token: &str,
) -> Result<Vec<String>> {
    let full_name = format!("{subdomain_label}.{base_domain}");
    let mut record_ids = Vec::new();

    // Create primary MX record (priority 10)
    let id = create_single_mx(zone_id, &full_name, mx_primary, 10, api_token).await?;
    record_ids.push(id);

    // Create secondary MX record (priority 20)
    let id = create_single_mx(zone_id, &full_name, mx_secondary, 20, api_token).await?;
    record_ids.push(id);

    Ok(record_ids)
}

/// Delete DNS records by their IDs.
pub async fn delete_dns_records(
    zone_id: &str,
    record_ids: &[String],
    api_token: &str,
) -> Result<()> {
    for record_id in record_ids {
        let url = format!("{CF_API_BASE}/zones/{zone_id}/dns_records/{record_id}");

        let headers = auth_headers(api_token)?;
        let mut init = RequestInit::new();
        init.with_method(Method::Delete).with_headers(headers);

        let req = Request::new_with_init(&url, &init)?;
        let resp = Fetch::Request(req).send().await?;

        if resp.status_code() != 200 {
            console_log!(
                "Warning: failed to delete DNS record {}: status {}",
                record_id,
                resp.status_code()
            );
        }
    }
    Ok(())
}

async fn create_single_mx(
    zone_id: &str,
    name: &str,
    content: &str,
    priority: u16,
    api_token: &str,
) -> Result<String> {
    let url = format!("{CF_API_BASE}/zones/{zone_id}/dns_records");
    let body = serde_json::json!({
        "type": "MX",
        "name": name,
        "content": content,
        "priority": priority,
        "ttl": 1,
    });

    let headers = auth_headers(api_token)?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(&body.to_string())));

    let req = Request::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(req).send().await?;
    let text = resp.text().await?;

    if resp.status_code() != 200 {
        return Err(Error::from(format!(
            "Cloudflare API error creating MX record: {}",
            text
        )));
    }

    let api_resp: CfApiResponse =
        serde_json::from_str(&text).map_err(|e| Error::from(e.to_string()))?;

    if !api_resp.success {
        let msg = api_resp
            .errors
            .first()
            .map(|e| e.message.clone())
            .unwrap_or_else(|| "Unknown error".into());
        return Err(Error::from(format!("Cloudflare DNS error: {msg}")));
    }

    api_resp
        .result
        .map(|r| r.id)
        .ok_or_else(|| Error::from("Missing record ID in Cloudflare response"))
}

fn auth_headers(api_token: &str) -> Result<Headers> {
    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {api_token}"))?;
    Ok(headers)
}
