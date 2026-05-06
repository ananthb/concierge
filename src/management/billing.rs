//! Management billing: pricing settings (per-currency rates, email pack
//! size, credit-purchase bounds).

use worker::*;

use crate::management::audit;
use crate::storage;
use crate::templates::management as tmpl;

pub async fn handle_billing(
    mut req: Request,
    _kv: &kv::KvStore,
    db: &D1Database,
    sub: &str,
    method: Method,
    actor_email: &str,
    base_url: &str,
) -> Result<Response> {
    let parts: Vec<&str> = sub
        .strip_prefix("billing")
        .unwrap_or("")
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let locale = crate::locale::Locale::from_request(&req);

    match (method, parts.as_slice()) {
        (Method::Get, []) => {
            let cfg = storage::get_pricing(db).await;
            Response::from_html(tmpl::billing_overview_html(
                actor_email,
                base_url,
                &locale,
                &cfg,
            ))
        }

        // Update pricing settings. Form posts a flat dict whose keys are
        // either `email_pack_size`, `min_credits`, `max_credits`, or
        // `<concept>__<currency>` (e.g. `unit_price_milli__INR`). We walk
        // the config + every known currency × concept and upsert anything
        // that's positive.
        (Method::Post, ["settings"]) => {
            let form: serde_json::Value = req.json().await?;
            let pick = |key: &str| -> Option<i64> {
                let v = form.get(key)?;
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
            };

            // Currency-agnostic settings. Existing snapshot drives the
            // defaults so a single-cell post doesn't clobber the other
            // two values when the form only carried one of them.
            let cfg_before = storage::get_pricing(db).await;
            let pack_size = pick("email_pack_size").unwrap_or(cfg_before.email_pack_size);
            let min_credits = pick("min_credits").unwrap_or(cfg_before.min_credits);
            let max_credits = pick("max_credits").unwrap_or(cfg_before.max_credits);

            if pack_size <= 0 {
                return Response::from_html(
                    r#"<div class="error">Addresses per pack must be positive.</div>"#.to_string(),
                );
            }
            if min_credits < 1 {
                return Response::from_html(
                    r#"<div class="error">Minimum credits must be at least 1.</div>"#.to_string(),
                );
            }
            if max_credits < min_credits {
                return Response::from_html(
                    r#"<div class="error">Maximum credits must be greater than or equal to the minimum.</div>"#
                        .to_string(),
                );
            }
            if max_credits > crate::billing::MAX_CREDITS_CEILING {
                return Response::from_html(format!(
                    r#"<div class="error">Maximum credits cannot exceed {}.</div>"#,
                    crate::billing::MAX_CREDITS_CEILING
                ));
            }
            storage::update_pricing_config(db, pack_size, min_credits, max_credits).await?;

            // Per-(concept, currency) cells. We accept any currency code
            // the form sends, so adding a currency client-side just works.
            let cfg = storage::get_pricing(db).await;
            let mut codes = cfg.currencies();
            // Form may also carry brand-new currency codes via the
            // `__currencies` JSON array (added by the "Add currency" UI).
            if let Some(extra) = form.get("__currencies").and_then(|v| v.as_array()) {
                for c in extra {
                    if let Some(s) = c.as_str() {
                        let s = s.to_uppercase();
                        if !codes.contains(&s) {
                            codes.push(s);
                        }
                    }
                }
            }
            for concept in storage::PricingConcept::ALL {
                for code in &codes {
                    let key = format!("{}__{}", concept.as_wire(), code);
                    if let Some(n) = pick(&key) {
                        if n <= 0 {
                            return Response::from_html(format!(
                                r#"<div class="error">Invalid value for {}: must be a positive integer.</div>"#,
                                crate::helpers::html_escape(&key),
                            ));
                        }
                        storage::upsert_pricing_amount(db, concept, code, n).await?;
                    }
                }
            }

            audit::log_action(
                db,
                actor_email,
                "update_pricing",
                "billing",
                None,
                Some(&form),
            )
            .await?;

            Response::from_html(
                r#"<div class="success">Pricing settings updated.</div>"#.to_string(),
            )
        }

        // Remove every row for a currency.
        (Method::Delete, ["currency", code]) => {
            let code = code.to_uppercase();
            storage::delete_pricing_currency(db, &code).await?;
            audit::log_action(
                db,
                actor_email,
                "delete_pricing_currency",
                "billing",
                Some(&code),
                None,
            )
            .await?;
            let cfg = storage::get_pricing(db).await;
            Response::from_html(tmpl::billing_overview_html(
                actor_email,
                base_url,
                &locale,
                &cfg,
            ))
        }

        _ => Response::error("Not Found", 404),
    }
}
