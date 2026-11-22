//! Billing: prepaid reply credit tracking + Razorpay payments.
//!
//! 1 credit = 1 reply (both static/canned and AI replies cost 1 credit).
//! Deduction happens BEFORE send (optimistic); on AI/send failure, the
//! credit is restored. Soonest-expiring credits are consumed first.

pub mod razorpay;
pub mod webhook;

use worker::*;

use crate::helpers::now_iso;
use crate::storage;
use crate::types::{CreditEntry, CreditSource, TenantBilling};

pub const MIN_CREDITS: i64 = 100;
pub const MAX_CREDITS: i64 = 100_000;
pub const DEFAULT_CURRENCY: &str = "INR";

/// Per-reply rate in milli-minor units (1/1000 of paise / cents).
pub fn unit_price_milli(currency: &str) -> i64 {
    match currency.trim().to_ascii_uppercase().as_str() {
        "INR" => 10_000, // ₹0.10 / reply
        "USD" => 100,    // $0.001 / reply
        _ => 0,
    }
}

/// Total price in the smallest currency unit (paise / cents) for a given credit amount.
pub fn calculate_total(credits: i64, milli_price: i64) -> i64 {
    (credits * milli_price + 500) / 1000
}

/// Pure credit deduction step against an in-memory `TenantBilling` ledger.
/// Returns `true` if the reply is allowed to proceed.
pub(crate) fn deduct_one(billing: &mut TenantBilling, metered: bool, now: &str) -> bool {
    prune_expired_at(billing, now);
    sort_credits(billing);

    let mut deducted = false;
    for entry in billing.credits.iter_mut() {
        if entry.amount > 0 {
            entry.amount -= 1;
            deducted = true;
            break;
        }
    }

    if !deducted && metered {
        billing.credits.retain(|e| e.amount > 0);
        return false;
    }

    billing.credits.retain(|e| e.amount > 0);
    billing.replies_used += 1;
    true
}

/// Pure credit restoration step against an in-memory `TenantBilling` ledger.
pub(crate) fn restore_one(billing: &mut TenantBilling, metered: bool, now: &str) {
    if metered {
        billing.credits.push(CreditEntry {
            amount: 1,
            source: CreditSource::Purchase,
            expires_at: None,
            granted_at: now.to_string(),
        });
    }
    billing.replies_used = (billing.replies_used - 1).max(0);
}

/// Try to deduct one reply credit before sending.
///
/// When `metered` is `true` (paid), a zero balance returns `Ok(false)` and
/// the reply is suppressed. When `false` (complimentary `Free` plan), we still
/// consume from any available balance and increment `replies_used`, returning `Ok(true)`.
pub async fn try_deduct(db: &D1Database, tenant_id: &str, metered: bool) -> Result<bool> {
    let mut billing = storage::get_tenant_billing(db, tenant_id).await?;
    let allowed = deduct_one(&mut billing, metered, &now_iso());
    storage::save_tenant_billing(db, tenant_id, &billing).await?;
    Ok(allowed)
}

/// Restore one reply credit after a failed generation/send.
pub async fn restore_credit(db: &D1Database, tenant_id: &str, metered: bool) -> Result<()> {
    let mut billing = storage::get_tenant_billing(db, tenant_id).await?;
    restore_one(&mut billing, metered, &now_iso());
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

/// Prepare billing for display: prune expired entries and sort soonest-expiring first.
pub fn refresh_billing(billing: &mut TenantBilling) {
    let now = now_iso();
    prune_expired_at(billing, &now);
    sort_credits(billing);
    billing.credits.retain(|e| e.amount > 0);
}

fn prune_expired_at(billing: &mut TenantBilling, now: &str) {
    billing.credits.retain(|e| match &e.expires_at {
        None => true,
        Some(exp) => exp.as_str() > now,
    });
}

fn sort_credits(billing: &mut TenantBilling) {
    billing
        .credits
        .sort_by(|a, b| match (&a.expires_at, &b.expires_at) {
            (Some(a_exp), Some(b_exp)) => a_exp.cmp(b_exp),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
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
    fn has_credits_and_total_remaining() {
        let empty = TenantBilling::default();
        assert!(!empty.has_credits());
        assert_eq!(empty.total_remaining(), 0);

        let b = make_billing(vec![
            entry(10, CreditSource::Purchase, None),
            entry(50, CreditSource::Grant, Some("2099-12-31T23:59:59Z")),
        ]);
        assert!(b.has_credits());
        assert_eq!(b.total_remaining(), 60);
    }

    #[test]
    fn deduct_and_restore_metered_round_trip() {
        let mut b = make_billing(vec![
            entry(1, CreditSource::Grant, Some("2027-01-01T00:00:00Z")),
            entry(2, CreditSource::Purchase, None),
        ]);
        let now = "2026-06-01T00:00:00Z";

        // Consumes soonest-expiring entry first
        assert!(deduct_one(&mut b, true, now));
        assert_eq!(b.total_remaining(), 2);
        assert_eq!(b.replies_used, 1);
        assert_eq!(b.credits.len(), 1);

        // Consume remaining two
        assert!(deduct_one(&mut b, true, now));
        assert!(deduct_one(&mut b, true, now));
        assert_eq!(b.total_remaining(), 0);
        assert_eq!(b.replies_used, 3);

        // Blocked at zero balance
        assert!(!deduct_one(&mut b, true, now));
        assert_eq!(b.replies_used, 3);

        // Restore puts 1 credit back and decrements replies_used
        restore_one(&mut b, true, now);
        assert_eq!(b.total_remaining(), 1);
        assert_eq!(b.replies_used, 2);
    }

    #[test]
    fn deduct_unmetered_allows_zero_balance_and_restore_does_not_mint() {
        let mut b = TenantBilling::default();
        let now = "2026-06-01T00:00:00Z";

        assert!(deduct_one(&mut b, false, now));
        assert_eq!(b.replies_used, 1);
        assert_eq!(b.total_remaining(), 0);

        restore_one(&mut b, false, now);
        assert_eq!(b.replies_used, 0);
        assert_eq!(b.total_remaining(), 0);
    }

    #[test]
    fn prune_expired_drops_old_before_deduct() {
        let mut b = make_billing(vec![entry(
            50,
            CreditSource::Grant,
            Some("2020-01-31T23:59:59Z"),
        )]);
        assert!(!deduct_one(&mut b, true, "2026-06-01T00:00:00Z"));
        assert!(b.credits.is_empty());
    }

    #[test]
    fn test_calculate_total() {
        assert_eq!(calculate_total(1, 10_000), 10);
        assert_eq!(calculate_total(100, 10_000), 1000); // ₹10.00
        assert_eq!(calculate_total(500, 10_000), 5000); // ₹50.00

        assert_eq!(calculate_total(1, 100), 0);
        assert_eq!(calculate_total(5, 100), 1);
        assert_eq!(calculate_total(500, 100), 50);
        assert_eq!(calculate_total(1000, 100), 100);
    }

    #[test]
    fn unit_price_milli_matches_supported_currencies() {
        assert_eq!(unit_price_milli(DEFAULT_CURRENCY), 10_000);
        assert_eq!(unit_price_milli("INR"), 10_000);
        assert_eq!(unit_price_milli("usd"), 100);
        assert_eq!(unit_price_milli("EUR"), 0);
    }
}
