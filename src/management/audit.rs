//! Audit logging for management actions.

use wasm_bindgen::JsValue;
use worker::*;

use crate::helpers::generate_id;

/// Log a management action to D1.
pub async fn log_action(
    db: &D1Database,
    actor_email: &str,
    action: &str,
    resource_type: &str,
    resource_id: Option<&str>,
    details: Option<&serde_json::Value>,
) -> Result<()> {
    let id = generate_id();
    let details_str = details
        .map(|d| serde_json::to_string(d).unwrap_or_else(|_| "{}".into()))
        .unwrap_or_else(|| "{}".into());

    let stmt = db.prepare(
        "INSERT INTO audit_log (id, actor_email, action, resource_type, resource_id, details)
         VALUES (?, ?, ?, ?, ?, ?)",
    );
    stmt.bind(&[
        id.as_str().into(),
        actor_email.into(),
        action.into(),
        resource_type.into(),
        resource_id.map(JsValue::from).unwrap_or(JsValue::null()),
        details_str.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Get recent audit log entries.
pub async fn get_audit_log(db: &D1Database, limit: u32) -> Result<Vec<serde_json::Value>> {
    let stmt = db.prepare("SELECT * FROM audit_log ORDER BY created_at DESC LIMIT ?");
    let result = stmt.bind(&[JsValue::from(limit as f64)])?.all().await?;
    result.results::<serde_json::Value>()
}
