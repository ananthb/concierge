//! Non-blocking Analytics Engine event metrics + structured console logging.
//!
//! Never logs or writes customer PII (phone numbers, message bodies, or names).

use worker::{console_log, AnalyticsEngineDataPointBuilder, Env};

/// Record an operational event via structured JSON `console_log!` and, when
/// the `EVENTS` Analytics Engine binding is configured, a non-blocking data point.
pub fn record(env: &Env, source: &str, event: &str, detail: &str) {
    let payload = serde_json::json!({
        "event": event,
        "source": source,
        "detail": detail,
    });
    console_log!("{}", payload);

    if let Ok(ae) = env.analytics_engine("EVENTS") {
        let _ = AnalyticsEngineDataPointBuilder::new()
            .indexes([source])
            .add_blob(event)
            .add_blob(detail)
            .add_double(1.0)
            .write_to(&ae);
    }
}
