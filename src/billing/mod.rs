//! Billing: reply credit tracking with expiry + Razorpay payments.
//!
//! Credits are stored as a ledger of entries, each with an optional expiry.
//! - Purchased credits never expire.
//! - Management grants (and recurring credit grants) expire after a
//!   configurable period (or never, when `expires_in_days = 0`).
//!
//! Deduction happens BEFORE send (optimistic). If send fails,
//! credits are restored. This prevents double-spend races.
//! Soonest-expiring credits are consumed first.

pub mod cadence;
pub mod razorpay;
pub mod webhook;

use worker::*;

use crate::helpers::{days_from_now, now_iso};
use crate::storage;
use crate::types::{CreditEntry, CreditSource, TenantBilling};

// Slider bounds for the credit purchase flow. These are pure UI knobs,
// not pricing. Keeping them as Rust constants because they don't make
// sense to edit at runtime from /manage.
pub const MIN_CREDITS: i64 = 500;
pub const MAX_CREDITS: i64 = 1_000_000;

// Per-currency pricing, the per-tenant free monthly credit grant, and
// the reply-email pack size all live in the `pricing_config` table.
// `storage::PricingConfig::default()` mirrors the SQL DEFAULT clauses
// so tests and host-side callers can reference the same numbers.

/// Total price in the smallest currency unit (paise / cents) for a given credit amount.
pub fn calculate_total(credits: i64, milli_price: i64) -> i64 {
    (credits * milli_price + 500) / 1000
}

/// Try to deduct one reply credit. Returns true if credit was available
/// and deducted. Returns false if out of credits.
/// Must be called BEFORE sending the reply.
pub async fn try_deduct(db: &D1Database, tenant_id: &str) -> Result<bool> {
    let mut billing = storage::get_tenant_billing(db, tenant_id).await?;
    prune_expired(&mut billing);
    sort_credits(&mut billing);

    let mut deducted = false;
    for entry in billing.credits.iter_mut() {
        if entry.amount > 0 {
            entry.amount -= 1;
            deducted = true;
            break;
        }
    }

    if !deducted {
        storage::save_tenant_billing(db, tenant_id, &billing).await?;
        return Ok(false);
    }

    billing.credits.retain(|e| e.amount > 0);
    billing.replies_used += 1;
    storage::save_tenant_billing(db, tenant_id, &billing).await?;
    Ok(true)
}

/// Restore one reply credit after a failed send.
/// Adds as a non-expiring purchase credit for simplicity.
pub async fn restore_credit(db: &D1Database, tenant_id: &str) -> Result<()> {
    let mut billing = storage::get_tenant_billing(db, tenant_id).await?;
    billing.credits.push(CreditEntry {
        amount: 1,
        source: CreditSource::Purchase,
        expires_at: None,
        granted_at: now_iso(),
    });
    billing.replies_used = (billing.replies_used - 1).max(0);
    storage::save_tenant_billing(db, tenant_id, &billing).await
}

/// Grant purchased credits (never expire).
pub async fn grant_purchased(db: &D1Database, tenant_id: &str, count: i64) -> Result<()> {
    if count <= 0 {
        return Err(Error::from("Credit count must be positive"));
    }
    let mut billing = storage::get_tenant_billing(db, tenant_id).await?;
    billing.credits.push(CreditEntry {
        amount: count,
        source: CreditSource::Purchase,
        expires_at: None,
        granted_at: now_iso(),
    });
    storage::save_tenant_billing(db, tenant_id, &billing).await
}

/// Grant credits with expiry (for management grants).
pub async fn grant_with_expiry(
    db: &D1Database,
    tenant_id: &str,
    count: i64,
    expires_in_days: i64,
) -> Result<()> {
    if count <= 0 {
        return Err(Error::from("Credit count must be positive"));
    }
    let mut billing = storage::get_tenant_billing(db, tenant_id).await?;
    billing.credits.push(CreditEntry {
        amount: count,
        source: CreditSource::Grant,
        expires_at: Some(days_from_now(expires_in_days)),
        granted_at: now_iso(),
    });
    storage::save_tenant_billing(db, tenant_id, &billing).await
}

/// Prepare billing for display. Prunes expired entries and sorts the
/// ledger so soonest-expiring credits are consumed first. Call this
/// before rendering billing UI.
pub fn refresh_billing(billing: &mut TenantBilling) {
    let now = now_iso();
    prune_expired_at(billing, &now);
    sort_credits(billing);
    billing.credits.retain(|e| e.amount > 0);
}

fn prune_expired(billing: &mut TenantBilling) {
    prune_expired_at(billing, &now_iso());
}

/// Remove expired credit entries.
fn prune_expired_at(billing: &mut TenantBilling, now: &str) {
    billing.credits.retain(|e| match &e.expires_at {
        None => true,
        Some(exp) => exp.as_str() > now,
    });
}

/// Sort credits by expiry: soonest first, never-expire last.
fn sort_credits(billing: &mut TenantBilling) {
    billing.credits.sort_by(|a, b| {
        match (&a.expires_at, &b.expires_at) {
            (Some(a_exp), Some(b_exp)) => a_exp.cmp(b_exp),
            (Some(_), None) => std::cmp::Ordering::Less, // expiring before never
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_billing(entries: Vec<CreditEntry>) -> TenantBilling {
        TenantBilling {
            credits: entries,
            replies_used: 0,
        }
    }

    fn entry(amount: i64, source: CreditSource, expires_at: Option<&str>) -> CreditEntry {
        CreditEntry {
            amount,
            source,
            expires_at: expires_at.map(|s| s.to_string()),
            granted_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn has_credits_positive() {
        let b = make_billing(vec![entry(5, CreditSource::Purchase, None)]);
        assert!(b.has_credits());
    }

    #[test]
    fn has_credits_empty() {
        let b = TenantBilling::default();
        assert!(!b.has_credits());
    }

    #[test]
    fn total_remaining_sums() {
        let b = make_billing(vec![
            entry(10, CreditSource::Purchase, None),
            entry(50, CreditSource::Grant, Some("2099-12-31T23:59:59Z")),
            entry(25, CreditSource::Grant, Some("2027-01-01T00:00:00Z")),
        ]);
        assert_eq!(b.total_remaining(), 85);
    }

    #[test]
    fn sort_credits_soonest_first() {
        let mut b = make_billing(vec![
            entry(10, CreditSource::Purchase, None),
            entry(5, CreditSource::Grant, Some("2027-06-01T00:00:00Z")),
            entry(100, CreditSource::Grant, Some("2026-04-30T23:59:59Z")),
        ]);
        sort_credits(&mut b);
        // First two are grants ordered by expiry; the never-expire purchase
        // sorts last.
        assert_eq!(b.credits[0].amount, 100);
        assert_eq!(b.credits[1].amount, 5);
        assert_eq!(b.credits[2].source, CreditSource::Purchase);
    }

    #[test]
    fn prune_expired_removes_old() {
        let mut b = make_billing(vec![
            entry(50, CreditSource::Grant, Some("2020-01-31T23:59:59Z")),
            entry(10, CreditSource::Purchase, None),
            entry(25, CreditSource::Grant, Some("2020-06-01T00:00:00Z")),
        ]);
        prune_expired_at(&mut b, "2026-04-19T12:00:00Z");
        assert_eq!(b.credits.len(), 1);
        assert_eq!(b.credits[0].source, CreditSource::Purchase);
    }

    #[test]
    fn prune_keeps_future() {
        let mut b = make_billing(vec![
            entry(50, CreditSource::Grant, Some("2099-12-31T23:59:59Z")),
            entry(10, CreditSource::Purchase, None),
        ]);
        prune_expired_at(&mut b, "2026-04-19T12:00:00Z");
        assert_eq!(b.credits.len(), 2);
    }

    #[test]
    fn test_calculate_total() {
        // 10 paise (10,000 milli-paise) per reply
        assert_eq!(calculate_total(1, 10_000), 10);
        assert_eq!(calculate_total(100, 10_000), 1000); // ₹10.00
        assert_eq!(calculate_total(500, 10_000), 5000); // ₹50.00

        // $0.001 (100 milli-cents) per reply
        assert_eq!(calculate_total(1, 100), 0); // rounds 0.1 to 0
        assert_eq!(calculate_total(5, 100), 1); // rounds 0.5 to 1
        assert_eq!(calculate_total(500, 100), 50); // $0.50
        assert_eq!(calculate_total(1000, 100), 100); // $1.00
    }
}
