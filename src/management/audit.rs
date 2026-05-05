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

/// Filtered audit log query. All three filter args are optional —
/// pass `""` to skip a filter. `actor` is a case-insensitive LIKE
/// (matches partial emails); `action` and `resource_type` are exact
/// matches against the wire vocab.
pub async fn search_audit_log(
    db: &D1Database,
    actor: &str,
    action: &str,
    resource_type: &str,
    limit: u32,
) -> Result<Vec<serde_json::Value>> {
    let actor = actor.trim();
    let action = action.trim();
    let resource_type = resource_type.trim();

    let mut where_clauses: Vec<&str> = Vec::new();
    let mut binds: Vec<JsValue> = Vec::new();

    let actor_pattern;
    if !actor.is_empty() {
        actor_pattern = format!(
            "%{}%",
            actor
                .replace('\\', r"\\")
                .replace('%', r"\%")
                .replace('_', r"\_")
        );
        where_clauses.push("actor_email LIKE ? ESCAPE '\\' COLLATE NOCASE");
        binds.push(actor_pattern.as_str().into());
    }
    if !action.is_empty() {
        where_clauses.push("action = ?");
        binds.push(action.into());
    }
    if !resource_type.is_empty() {
        where_clauses.push("resource_type = ?");
        binds.push(resource_type.into());
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_clauses.join(" AND "))
    };
    let sql = format!("SELECT * FROM audit_log{where_sql} ORDER BY created_at DESC LIMIT ?");
    binds.push(JsValue::from(limit as f64));

    let stmt = db.prepare(&sql);
    let result = stmt.bind(&binds)?.all().await?;
    result.results::<serde_json::Value>()
}
