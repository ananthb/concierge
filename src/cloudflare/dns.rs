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
struct CfListResponse {
    success: bool,
    #[serde(default)]
    result: Vec<CfMxRecord>,
}

#[derive(Deserialize)]
struct CfApiError {
    message: String,
}

#[derive(Deserialize)]
struct CfDnsRecord {
    id: String,
}

#[derive(Deserialize)]
struct CfMxRecord {
    content: String,
    priority: u16,
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

/// Create DNS records for a subdomain: MX records (for email) + a proxied A record (for HTTP redirect).
/// Returns all DNS record IDs on success.
pub async fn create_subdomain_records(
    zone_id: &str,
    subdomain_label: &str,
    base_domain: &str,
    api_token: &str,
) -> Result<Vec<String>> {
    // Discover MX servers from the apex domain
    let apex_mx = get_mx_records(zone_id, base_domain, api_token).await?;
    if apex_mx.is_empty() {
        return Err(Error::from(
            "No MX records found on apex domain. Enable Cloudflare Email Routing first.",
        ));
    }

    let full_name = format!("{subdomain_label}.{base_domain}");
    let mut record_ids = Vec::new();

    // MX records for email routing
    for mx in &apex_mx {
        let id = create_single_mx(zone_id, &full_name, &mx.content, mx.priority, api_token).await?;
        record_ids.push(id);
    }

    // Proxied A + AAAA records for HTTP (redirects to main site)
    let web_ids = create_proxied_web_records(zone_id, &full_name, api_token).await?;
    record_ids.extend(web_ids);

    Ok(record_ids)
}

/// Create proxied A + AAAA records so HTTP requests reach the Worker.
async fn create_proxied_web_records(
    zone_id: &str,
    name: &str,
    api_token: &str,
) -> Result<Vec<String>> {
    let mut ids = Vec::new();

    // Proxied A record (dummy IP — CF proxies to Worker)
    let id = create_dns_record(
        zone_id,
        &serde_json::json!({
            "type": "A",
            "name": name,
            "content": "192.0.2.1",
            "proxied": true,
            "ttl": 1,
        }),
        api_token,
    )
    .await?;
    ids.push(id);

    // Proxied AAAA record
    let id = create_dns_record(
        zone_id,
        &serde_json::json!({
            "type": "AAAA",
            "name": name,
            "content": "100::",
            "proxied": true,
            "ttl": 1,
        }),
        api_token,
    )
    .await?;
    ids.push(id);

    Ok(ids)
}

async fn create_dns_record(
    zone_id: &str,
    body: &serde_json::Value,
    api_token: &str,
) -> Result<String> {
    let url = format!("{CF_API_BASE}/zones/{zone_id}/dns_records");

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
            "Cloudflare API error creating DNS record: {}",
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

/// List MX records for a given name in the zone.
async fn get_mx_records(zone_id: &str, name: &str, api_token: &str) -> Result<Vec<CfMxRecord>> {
    let url = format!(
        "{CF_API_BASE}/zones/{zone_id}/dns_records?type=MX&name={name}",
        name = urlencoding::encode(name),
    );

    let headers = auth_headers(api_token)?;
    let mut init = RequestInit::new();
    init.with_method(Method::Get).with_headers(headers);

    let req = Request::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(req).send().await?;
    let text = resp.text().await?;

    if resp.status_code() != 200 {
        return Err(Error::from(format!("Failed to list MX records: {}", text)));
    }

    let api_resp: CfListResponse =
        serde_json::from_str(&text).map_err(|e| Error::from(e.to_string()))?;

    if !api_resp.success {
        return Err(Error::from("Cloudflare API returned success=false"));
    }

    Ok(api_resp.result)
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
    create_dns_record(
        zone_id,
        &serde_json::json!({
            "type": "MX",
            "name": name,
            "content": content,
            "priority": priority,
            "ttl": 1,
        }),
        api_token,
    )
    .await
}

const WORKER_A: &str = "192.0.2.1";
const WORKER_AAAA: &str = "100::";

#[derive(Deserialize)]
struct CfRecordFull {
    id: String,
    #[serde(rename = "type")]
    record_type: String,
    content: String,
    proxied: Option<bool>,
}

#[derive(Deserialize)]
struct CfListFullResponse {
    success: bool,
    #[serde(default)]
    result: Vec<CfRecordFull>,
}

/// Verify the apex email domain has exactly the correct proxied A + AAAA records.
/// Deletes incorrect records and creates missing ones.
pub async fn verify_apex_web_records(
    zone_id: &str,
    base_domain: &str,
    api_token: &str,
) -> Result<()> {
    // List all A and AAAA records for the apex
    let url = format!(
        "{CF_API_BASE}/zones/{zone_id}/dns_records?name={name}",
        name = urlencoding::encode(base_domain),
    );

    let headers = auth_headers(api_token)?;
    let mut init = RequestInit::new();
    init.with_method(Method::Get).with_headers(headers);

    let req = Request::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(req).send().await?;
    let text = resp.text().await?;

    let list: CfListFullResponse =
        serde_json::from_str(&text).map_err(|e| Error::from(e.to_string()))?;

    if !list.success {
        return Err(Error::from("Failed to list DNS records for apex domain"));
    }

    let mut has_correct_a = false;
    let mut has_correct_aaaa = false;

    for record in &list.result {
        match record.record_type.as_str() {
            "A" => {
                if record.content == WORKER_A && record.proxied == Some(true) {
                    has_correct_a = true;
                } else {
                    // Wrong A record — delete it
                    console_log!(
                        "Deleting incorrect A record {} ({})",
                        record.id,
                        record.content
                    );
                    let _ = delete_dns_records(zone_id, &[record.id.clone()], api_token).await;
                }
            }
            "AAAA" => {
                if record.content == WORKER_AAAA && record.proxied == Some(true) {
                    has_correct_aaaa = true;
                } else {
                    // Wrong AAAA record — delete it
                    console_log!(
                        "Deleting incorrect AAAA record {} ({})",
                        record.id,
                        record.content
                    );
                    let _ = delete_dns_records(zone_id, &[record.id.clone()], api_token).await;
                }
            }
            _ => {}
        }
    }

    // Create missing records
    if !has_correct_a {
        create_dns_record(
            zone_id,
            &serde_json::json!({
                "type": "A",
                "name": base_domain,
                "content": WORKER_A,
                "proxied": true,
                "ttl": 1,
            }),
            api_token,
        )
        .await?;
        console_log!("Created A record for {base_domain}");
    }

    if !has_correct_aaaa {
        create_dns_record(
            zone_id,
            &serde_json::json!({
                "type": "AAAA",
                "name": base_domain,
                "content": WORKER_AAAA,
                "proxied": true,
                "ttl": 1,
            }),
            api_token,
        )
        .await?;
        console_log!("Created AAAA record for {base_domain}");
    }

    Ok(())
}

fn auth_headers(api_token: &str) -> Result<Headers> {
    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {api_token}"))?;
    Ok(headers)
}
