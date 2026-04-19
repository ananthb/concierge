//! Discord interaction endpoint — signature verification + dispatch.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use worker::*;

use crate::types::DiscordInteraction;

use super::{commands, components};

/// Handle POST /discord/interactions
pub async fn handle_interaction(mut req: Request, env: Env) -> Result<Response> {
    let public_key = env
        .secret("DISCORD_PUBLIC_KEY")
        .map(|s| s.to_string())
        .unwrap_or_default();

    if public_key.is_empty() {
        return Response::error("DISCORD_PUBLIC_KEY not configured", 500);
    }

    let signature = req
        .headers()
        .get("X-Signature-Ed25519")?
        .unwrap_or_default();
    let timestamp = req
        .headers()
        .get("X-Signature-Timestamp")?
        .unwrap_or_default();
    let body = req.text().await?;

    if !verify_ed25519(&public_key, &signature, &timestamp, &body).await? {
        return Response::error("Invalid signature", 401);
    }

    let interaction: DiscordInteraction = serde_json::from_str(&body)
        .map_err(|e| Error::from(format!("Failed to parse interaction: {e}")))?;

    match interaction.interaction_type {
        // PING — Discord endpoint verification
        1 => Response::from_json(&serde_json::json!({"type": 1})),

        // APPLICATION_COMMAND — slash commands
        2 => commands::handle_command(&interaction, &env).await,

        // MESSAGE_COMPONENT — buttons
        3 => components::handle_component(&interaction, &env).await,

        // MODAL_SUBMIT — modal form responses
        5 => components::handle_modal_submit(&interaction, &env).await,

        _ => Response::error("Unknown interaction type", 400),
    }
}

/// Verify Discord Ed25519 signature using Web Crypto API.
/// Cloudflare Workers use the "NODE-ED25519" algorithm name.
async fn verify_ed25519(
    public_key_hex: &str,
    signature_hex: &str,
    timestamp: &str,
    body: &str,
) -> Result<bool> {
    let public_key_bytes = hex_decode(public_key_hex)?;
    let signature_bytes = hex_decode(signature_hex)?;

    let message = format!("{timestamp}{body}");
    let message_bytes = message.as_bytes();

    let crypto = get_subtle()?;

    // Import the public key
    let algorithm = js_sys::Object::new();
    js_sys::Reflect::set(
        &algorithm,
        &JsValue::from_str("name"),
        &JsValue::from_str("NODE-ED25519"),
    )
    .map_err(|_| Error::from("Failed to set algorithm"))?;
    js_sys::Reflect::set(
        &algorithm,
        &JsValue::from_str("namedCurve"),
        &JsValue::from_str("NODE-ED25519"),
    )
    .map_err(|_| Error::from("Failed to set namedCurve"))?;

    let key_usages = js_sys::Array::new();
    key_usages.push(&JsValue::from_str("verify"));

    let key_data = js_sys::Uint8Array::from(public_key_bytes.as_slice());

    let key_promise = crypto
        .import_key_with_object("raw", &key_data.buffer(), &algorithm, true, &key_usages)
        .map_err(|e| Error::from(format!("Key import failed: {e:?}")))?;

    let crypto_key: web_sys::CryptoKey = wasm_bindgen_futures::JsFuture::from(key_promise)
        .await
        .map_err(|e| Error::from(format!("Key import await failed: {e:?}")))?
        .dyn_into()
        .map_err(|_| Error::from("Failed to cast CryptoKey"))?;

    // Verify the signature
    let sig_array = js_sys::Uint8Array::from(signature_bytes.as_slice());
    let msg_array = js_sys::Uint8Array::from(message_bytes);

    let verify_promise = crypto
        .verify_with_object_and_buffer_source_and_buffer_source(
            &algorithm,
            &crypto_key,
            &sig_array,
            &msg_array,
        )
        .map_err(|e| Error::from(format!("Verify call failed: {e:?}")))?;

    let result = wasm_bindgen_futures::JsFuture::from(verify_promise)
        .await
        .map_err(|e| Error::from(format!("Verify await failed: {e:?}")))?;

    Ok(result.as_bool().unwrap_or(false))
}

fn get_subtle() -> Result<web_sys::SubtleCrypto> {
    let global = js_sys::global();
    let crypto = js_sys::Reflect::get(&global, &JsValue::from_str("crypto"))
        .map_err(|_| Error::from("Failed to get crypto"))?;
    let crypto: web_sys::Crypto = crypto
        .dyn_into()
        .map_err(|_| Error::from("Failed to cast to Crypto"))?;
    Ok(crypto.subtle())
}

fn hex_decode(hex: &str) -> Result<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return Err(Error::from("Invalid hex: odd length"));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| Error::from("Invalid hex character"))
        })
        .collect()
}
