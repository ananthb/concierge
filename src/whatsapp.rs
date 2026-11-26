use worker::*;

/// Send a WhatsApp message via Meta Graph API
pub async fn send_whatsapp_message(
    access_token: &str,
    phone_number_id: &str,
    to: &str,
    text: &str,
) -> Result<()> {
    let url = format!(
        "https://graph.facebook.com/{}/{}/messages",
        crate::META_API_VERSION,
        phone_number_id
    );

    let payload = serde_json::json!({
        "messaging_product": "whatsapp",
        "to": to,
        "type": "text",
        "text": {
            "body": text
        }
    });

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", access_token))?;
    headers.set("Content-Type", "application/json")?;

    let request = Request::new_with_init(
        &url,
        RequestInit::new()
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(wasm_bindgen::JsValue::from_str(&payload.to_string()))),
    )?;

    let mut response = Fetch::Request(request).send().await?;

    if !response.status_code().to_string().starts_with('2') {
        let status = response.status_code();
        let err_body = response.text().await.unwrap_or_default();
        let snippet: String = err_body.chars().take(240).collect();
        return Err(Error::from(format!(
            "WhatsApp API error (HTTP {status}): {snippet}"
        )));
    }

    Ok(())
}
