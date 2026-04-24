//! Billing — reply credit tracking with expiry + Razorpay payments.
//!
//! Credits are stored as a ledger of entries, each with an optional expiry.
//! - Purchased credits never expire.
//! - Free monthly credits (100/month) expire at month's end.
//! - Management grants expire after a configurable period (default 1 year).
//!
//! Deduction happens BEFORE send (optimistic). If send fails,
//! credits are restored. This prevents double-spend races.
//! Soonest-expiring credits are consumed first.

pub mod razorpay;
pub mod webhook;

use worker::*;

use crate::helpers::{current_month, days_from_now, end_of_month, now_iso};
use crate::storage;
use crate::types::{CreditEntry, CreditSource, TenantBilling};

const FREE_MONTHLY_AMOUNT: i64 = 100;

// ============================================================================
// Pricing
// ============================================================================
//
// Flat per-reply rate. No tiers, no discounts. The user picks any amount.
pub const UNIT_PRICE_PAISE: i64 = 200; // ₹2.00 per reply
pub const UNIT_PRICE_CENTS: i64 = 2; //   $0.02 per reply
pub const MIN_CREDITS: i64 = 100;
pub const MAX_CREDITS: i64 = 1_000_000;

/// Per-reply price in the smallest currency unit (paise / cents).
pub fn unit_price(currency: &str) -> i64 {
    if currency == "USD" {
        UNIT_PRICE_CENTS
    } else {
        UNIT_PRICE_PAISE
    }
}

/// Try to deduct one reply credit. Returns true if credit was available
/// and deducted. Returns false if out of credits.
/// Must be called BEFORE sending the reply.
pub async fn try_deduct(db: &D1Database, tenant_id: &str) -> Result<bool> {
    let mut billing = storage::get_tenant_billing(db, tenant_id).await?;
    ensure_free_monthly(&mut billing);
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

/// Prepare billing for display — ensures free monthly credits and prunes expired.
/// Call this before rendering billing UI.
pub fn refresh_billing(billing: &mut TenantBilling) {
    let now = now_iso();
    let month = current_month();
    let eom = end_of_month();
    ensure_free_monthly_at(billing, &month, &eom, &now);
    prune_expired_at(billing, &now);
    sort_credits(billing);
    billing.credits.retain(|e| e.amount > 0);
}

fn prune_expired(billing: &mut TenantBilling) {
    prune_expired_at(billing, &now_iso());
}

fn ensure_free_monthly(billing: &mut TenantBilling) {
    let now = now_iso();
    let month = current_month();
    let eom = end_of_month();
    ensure_free_monthly_at(billing, &month, &eom, &now);
}

/// Remove expired credit entries.
fn prune_expired_at(billing: &mut TenantBilling, now: &str) {
    billing.credits.retain(|e| match &e.expires_at {
        None => true,
        Some(exp) => exp.as_str() > now,
    });
}

/// Grant 100 free monthly credits if not already granted this month.
/// Removes any leftover free monthly entries from previous months.
fn ensure_free_monthly_at(billing: &mut TenantBilling, month: &str, eom: &str, now: &str) {
    if billing.free_month.as_deref() == Some(month) {
        return;
    }
    // Remove any old free monthly entries
    billing
        .credits
        .retain(|e| e.source != CreditSource::FreeMonthly);
    // Add new free monthly credits
    billing.credits.push(CreditEntry {
        amount: FREE_MONTHLY_AMOUNT,
        source: CreditSource::FreeMonthly,
        expires_at: Some(eom.to_string()),
        granted_at: now.to_string(),
    });
    billing.free_month = Some(month.to_string());
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
            free_month: None,
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
            entry(50, CreditSource::FreeMonthly, Some("2099-12-31T23:59:59Z")),
            entry(25, CreditSource::Grant, Some("2027-01-01T00:00:00Z")),
        ]);
        assert_eq!(b.total_remaining(), 85);
    }

    #[test]
    fn sort_credits_soonest_first() {
        let mut b = make_billing(vec![
            entry(10, CreditSource::Purchase, None),
            entry(5, CreditSource::Grant, Some("2027-06-01T00:00:00Z")),
            entry(100, CreditSource::FreeMonthly, Some("2026-04-30T23:59:59Z")),
        ]);
        sort_credits(&mut b);
        assert_eq!(b.credits[0].source, CreditSource::FreeMonthly);
        assert_eq!(b.credits[1].source, CreditSource::Grant);
        assert_eq!(b.credits[2].source, CreditSource::Purchase);
    }

    #[test]
    fn prune_expired_removes_old() {
        let mut b = make_billing(vec![
            entry(50, CreditSource::FreeMonthly, Some("2020-01-31T23:59:59Z")),
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
    fn ensure_free_monthly_grants_once() {
        let mut b = TenantBilling::default();
        ensure_free_monthly_at(
            &mut b,
            "2026-04",
            "2026-04-30T23:59:59Z",
            "2026-04-19T00:00:00Z",
        );
        assert_eq!(b.free_month, Some("2026-04".to_string()));
        assert_eq!(b.credits.len(), 1);
        assert_eq!(b.credits[0].amount, 100);
        assert_eq!(b.credits[0].source, CreditSource::FreeMonthly);
        assert_eq!(
            b.credits[0].expires_at,
            Some("2026-04-30T23:59:59Z".to_string())
        );

        // Second call same month — no-op
        ensure_free_monthly_at(
            &mut b,
            "2026-04",
            "2026-04-30T23:59:59Z",
            "2026-04-19T00:00:00Z",
        );
        assert_eq!(b.credits.len(), 1);
    }

    #[test]
    fn ensure_free_monthly_replaces_old() {
        let mut b = TenantBilling {
            credits: vec![entry(
                30,
                CreditSource::FreeMonthly,
                Some("2020-01-31T23:59:59Z"),
            )],
            free_month: Some("2020-01".to_string()),
            replies_used: 0,
        };
        ensure_free_monthly_at(
            &mut b,
            "2026-04",
            "2026-04-30T23:59:59Z",
            "2026-04-19T00:00:00Z",
        );
        assert_eq!(b.credits.len(), 1);
        assert_eq!(b.credits[0].amount, 100); // fresh 100, not old 30
        assert_eq!(b.free_month, Some("2026-04".to_string()));
    }

    #[test]
    fn ensure_free_monthly_preserves_other_credits() {
        let mut b = TenantBilling {
            credits: vec![
                entry(30, CreditSource::FreeMonthly, Some("2020-01-31T23:59:59Z")),
                entry(500, CreditSource::Purchase, None),
            ],
            free_month: Some("2020-01".to_string()),
            replies_used: 10,
        };
        ensure_free_monthly_at(
            &mut b,
            "2026-04",
            "2026-04-30T23:59:59Z",
            "2026-04-19T00:00:00Z",
        );
        assert_eq!(b.credits.len(), 2);
        assert_eq!(b.credits[0].source, CreditSource::Purchase);
        assert_eq!(b.credits[0].amount, 500);
        assert_eq!(b.credits[1].source, CreditSource::FreeMonthly);
        assert_eq!(b.credits[1].amount, 100);
    }
}
