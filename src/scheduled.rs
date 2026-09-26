use worker::*;

/// Hourly tick. Re-rolls demo personas when the operator-configured
/// regen cadence has elapsed; cheap when not due (one KV read).
///
/// The only cron left. The 15-minute approval-digest sweep went with the
/// approval queue, and the daily Instagram token refresh went with the
/// Instagram channel. Mirror this string in `wrangler.toml`.
pub const CRON_HOURLY_TICK: &str = "0 * * * *";

pub async fn handle_scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let cron = event.cron();
    console_log!("Scheduled job started: {cron}");

    match cron.as_str() {
        CRON_HOURLY_TICK => {
            if let Err(e) = roll_demo_personas_if_due(&env).await {
                console_log!("Demo persona regen error: {:?}", e);
            }
        }
        other => console_log!("Unknown cron schedule: {other}"),
    }

    console_log!("Scheduled job completed: {cron}");
}

/// Re-roll the stored demo personas when:
///   - the demo is enabled,
///   - the operator-configured regen cadence (>0) has elapsed since
///     the last roll,
///   - or the store is empty (nothing to compare against).
///
/// Pure no-op when the cadence is 0 (manual re-rolls only).
async fn roll_demo_personas_if_due(env: &Env) -> Result<()> {
    let kv = env.kv("KV")?;
    let cfg = crate::storage::get_demo_config(&kv)
        .await
        .unwrap_or_default();
    if !cfg.enabled || cfg.regeneration_cadence_mins == 0 {
        return Ok(());
    }

    let stored = crate::storage::get_stored_demo_personas(&kv)
        .await
        .ok()
        .flatten();
    let due = match stored.as_ref() {
        None => true,
        Some(s) => {
            let then_ms = js_sys::Date::parse(&s.generated_at);
            if then_ms.is_nan() {
                true // unparseable timestamp: treat as due so the next tick rolls
            } else {
                let now_ms = js_sys::Date::now();
                let elapsed_mins = ((now_ms - then_ms) / 60_000.0) as i64;
                elapsed_mins >= cfg.regeneration_cadence_mins as i64
            }
        }
    };
    if !due {
        return Ok(());
    }

    let db = env.d1("DB")?;
    match crate::handlers::demo_personas_list::regenerate_and_store(
        env,
        &kv,
        &db,
        &cfg.persona_generation_prompt,
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(msg) => {
            console_log!("demo persona cron regen failed: {msg}");
            Ok(())
        }
    }
}
