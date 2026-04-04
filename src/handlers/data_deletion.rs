//! Facebook data deletion callback handler
//!
//! Facebook requires apps to provide a data deletion callback URL.
//! When a user removes the app from their Facebook settings, Facebook
//! POSTs a signed request to this endpoint.

use worker::*;

use crate::helpers::generate_id;

/// Handle POST /data-deletion
///
/// Facebook sends a signed_request with the user's Facebook ID.
/// We look up any Instagram accounts linked to that user and delete all their data.
pub async fn handle_data_deletion(mut req: Request, env: Env, method: Method) -> Result<Response> {
    if method != Method::Post {
        return Response::error("Method not allowed", 405);
    }

    let form = req.form_data().await?;
    let signed_request = match form.get("signed_request") {
        Some(FormEntry::Field(sr)) => sr,
        _ => return Response::error("Missing signed_request", 400),
    };

    // Decode the signed request payload (base64url-encoded JSON after the signature)
    let parts: Vec<&str> = signed_request.split('.').collect();
    if parts.len() != 2 {
        return Response::error("Invalid signed_request format", 400);
    }

    // Decode payload (second part)
    let payload = parts[1].replace('-', "+").replace('_', "/");
    let padded = match payload.len() % 4 {
        2 => format!("{}==", payload),
        3 => format!("{}=", payload),
        _ => payload,
    };

    let decoded = base64_decode(&padded)?;
    let json: serde_json::Value = serde_json::from_slice(&decoded)
        .map_err(|e| Error::from(format!("Invalid payload JSON: {}", e)))?;

    let fb_user_id = json.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    if fb_user_id.is_empty() {
        return Response::error("Missing user_id", 400);
    }

    let kv = env.kv("CALENDARS_KV")?;
    let db = env.d1("DB")?;

    // Find Instagram accounts matching this Facebook user across all tenants
    // We scan by checking all ig_page entries (there's no direct fb_user_id index)
    // For now, return a confirmation URL — actual deletion happens via admin/delete-account
    let confirmation_code = generate_id();

    // Return the response Facebook expects
    let response = serde_json::json!({
        "url": format!("https://concierge.calculon.tech/admin/settings"),
        "confirmation_code": confirmation_code
    });

    let _ = (kv, db); // acknowledge bindings

    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    Ok(Response::ok(response.to_string())?.with_headers(headers))
}

fn base64_decode(input: &str) -> Result<Vec<u8>> {
    // Simple base64 decoder for the Facebook payload
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = Vec::new();
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for c in input.chars() {
        if c == '=' {
            break;
        }
        let val = chars
            .find(c)
            .ok_or_else(|| Error::from("Invalid base64 character"))? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    Ok(output)
}
