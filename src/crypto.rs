use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use worker::*;

use crate::types::InstagramToken;

const ALGORITHM: &str = "AES-GCM";
const IV_LENGTH: usize = 12;
const TAG_LENGTH: u8 = 128;

/// Encrypt an Instagram token for storage
pub async fn encrypt_token(token: &InstagramToken, key: &str) -> Result<String> {
    let plaintext = serde_json::to_string(token)
        .map_err(|e| Error::from(format!("Failed to serialize token: {}", e)))?;

    let crypto = get_crypto()?;
    let key_bytes = hex_decode(key)?;
    let crypto_key = import_key(&crypto, &key_bytes).await?;

    // Generate random IV
    let iv = generate_iv()?;

    // Encrypt
    let ciphertext = encrypt(&crypto, &crypto_key, &iv, plaintext.as_bytes()).await?;

    // Combine IV + ciphertext and encode as hex
    let mut combined = iv.to_vec();
    combined.extend_from_slice(&ciphertext);

    Ok(hex_encode(&combined))
}

/// Decrypt an Instagram token from storage
pub async fn decrypt_token(encrypted: &str, key: &str) -> Result<InstagramToken> {
    let combined = hex_decode(encrypted)?;

    if combined.len() < IV_LENGTH {
        return Err(Error::from("Invalid encrypted data: too short"));
    }

    let iv = &combined[..IV_LENGTH];
    let ciphertext = &combined[IV_LENGTH..];

    let crypto = get_crypto()?;
    let key_bytes = hex_decode(key)?;
    let crypto_key = import_key(&crypto, &key_bytes).await?;

    let plaintext = decrypt(&crypto, &crypto_key, iv, ciphertext).await?;

    let token: InstagramToken = serde_json::from_slice(&plaintext)
        .map_err(|e| Error::from(format!("Failed to deserialize token: {}", e)))?;

    Ok(token)
}

// ============================================================================
// Internal crypto helpers using Web Crypto API
// ============================================================================

fn get_crypto() -> Result<web_sys::SubtleCrypto> {
    let global = js_sys::global();
    let crypto = js_sys::Reflect::get(&global, &JsValue::from_str("crypto"))
        .map_err(|_| Error::from("Failed to get crypto"))?;
    let crypto: web_sys::Crypto = crypto
        .dyn_into()
        .map_err(|_| Error::from("Failed to cast to Crypto"))?;
    Ok(crypto.subtle())
}

async fn import_key(
    crypto: &web_sys::SubtleCrypto,
    key_bytes: &[u8],
) -> Result<web_sys::CryptoKey> {
    let algorithm = js_sys::Object::new();
    js_sys::Reflect::set(
        &algorithm,
        &JsValue::from_str("name"),
        &JsValue::from_str(ALGORITHM),
    )
    .map_err(|_| Error::from("Failed to set algorithm name"))?;

    let key_usages = js_sys::Array::new();
    key_usages.push(&JsValue::from_str("encrypt"));
    key_usages.push(&JsValue::from_str("decrypt"));

    let key_data = js_sys::Uint8Array::from(key_bytes);

    let promise = crypto
        .import_key_with_object("raw", &key_data.buffer(), &algorithm, false, &key_usages)
        .map_err(|e| Error::from(format!("Failed to import key: {:?}", e)))?;

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| Error::from(format!("Key import failed: {:?}", e)))?;

    result
        .dyn_into()
        .map_err(|_| Error::from("Failed to cast to CryptoKey"))
}

async fn encrypt(
    crypto: &web_sys::SubtleCrypto,
    key: &web_sys::CryptoKey,
    iv: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>> {
    let algorithm = js_sys::Object::new();
    js_sys::Reflect::set(
        &algorithm,
        &JsValue::from_str("name"),
        &JsValue::from_str(ALGORITHM),
    )
    .map_err(|_| Error::from("Failed to set algorithm name"))?;
    js_sys::Reflect::set(
        &algorithm,
        &JsValue::from_str("iv"),
        &js_sys::Uint8Array::from(iv),
    )
    .map_err(|_| Error::from("Failed to set IV"))?;
    js_sys::Reflect::set(
        &algorithm,
        &JsValue::from_str("tagLength"),
        &JsValue::from(TAG_LENGTH),
    )
    .map_err(|_| Error::from("Failed to set tag length"))?;

    let data = js_sys::Uint8Array::from(plaintext);

    let promise = crypto
        .encrypt_with_object_and_buffer_source(&algorithm, key, &data)
        .map_err(|e| Error::from(format!("Encrypt failed: {:?}", e)))?;

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| Error::from(format!("Encryption failed: {:?}", e)))?;

    let array_buffer: js_sys::ArrayBuffer = result
        .dyn_into()
        .map_err(|_| Error::from("Failed to cast to ArrayBuffer"))?;

    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    Ok(uint8_array.to_vec())
}

async fn decrypt(
    crypto: &web_sys::SubtleCrypto,
    key: &web_sys::CryptoKey,
    iv: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>> {
    let algorithm = js_sys::Object::new();
    js_sys::Reflect::set(
        &algorithm,
        &JsValue::from_str("name"),
        &JsValue::from_str(ALGORITHM),
    )
    .map_err(|_| Error::from("Failed to set algorithm name"))?;
    js_sys::Reflect::set(
        &algorithm,
        &JsValue::from_str("iv"),
        &js_sys::Uint8Array::from(iv),
    )
    .map_err(|_| Error::from("Failed to set IV"))?;
    js_sys::Reflect::set(
        &algorithm,
        &JsValue::from_str("tagLength"),
        &JsValue::from(TAG_LENGTH),
    )
    .map_err(|_| Error::from("Failed to set tag length"))?;

    let data = js_sys::Uint8Array::from(ciphertext);

    let promise = crypto
        .decrypt_with_object_and_buffer_source(&algorithm, key, &data)
        .map_err(|e| Error::from(format!("Decrypt failed: {:?}", e)))?;

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| Error::from(format!("Decryption failed: {:?}", e)))?;

    let array_buffer: js_sys::ArrayBuffer = result
        .dyn_into()
        .map_err(|_| Error::from("Failed to cast to ArrayBuffer"))?;

    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    Ok(uint8_array.to_vec())
}

fn generate_iv() -> Result<[u8; IV_LENGTH]> {
    let mut iv = [0u8; IV_LENGTH];
    getrandom::getrandom(&mut iv)
        .map_err(|e| Error::from(format!("Failed to generate IV: {}", e)))?;
    Ok(iv)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(hex: &str) -> Result<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return Err(Error::from("Invalid hex string: odd length"));
    }

    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| Error::from("Invalid hex character"))
        })
        .collect()
}

/// Compute HMAC-SHA256 and return as hex
pub fn hmac_sha256_hex(key: &[u8], data: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC key length is valid");
    mac.update(data);
    let result = mac.finalize();
    hex_encode(&result.into_bytes())
}

/// Verify a Meta webhook signature (X-Hub-Signature-256 header)
/// Returns true if signature is valid
pub fn verify_meta_signature(app_secret: &str, body: &[u8], signature_header: &str) -> bool {
    let expected = signature_header.strip_prefix("sha256=").unwrap_or("");
    if expected.is_empty() {
        return false;
    }
    let computed = hmac_sha256_hex(app_secret.as_bytes(), body);
    // Constant-time comparison
    if computed.len() != expected.len() {
        return false;
    }
    computed
        .bytes()
        .zip(expected.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}
