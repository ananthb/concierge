//! Billing — reply credit tracking + Razorpay payments.
//!
//! Credits = reply count. Management grants free credits.
//! Clients can also buy packs via Razorpay.
//! When credits hit 0, auto-replies stop.
//!
//! Deduction happens BEFORE send (optimistic). If send fails,
//! credits are restored. This prevents double-spend races.

pub mod razorpay;
pub mod webhook;

use worker::*;

use crate::storage;

/// Try to deduct one reply credit. Returns true if credit was available
/// and deducted. Returns false if out of credits.
/// Must be called BEFORE sending the reply.
pub async fn try_deduct(kv: &kv::KvStore, tenant_id: &str) -> Result<bool> {
    let mut billing = storage::get_tenant_billing(kv, tenant_id).await?;
    if billing.replies_remaining <= 0 {
        return Ok(false);
    }
    billing.replies_remaining -= 1;
    billing.replies_used += 1;
    storage::save_tenant_billing(kv, tenant_id, &billing).await?;
    Ok(true)
}

/// Restore one reply credit after a failed send.
pub async fn restore_credit(kv: &kv::KvStore, tenant_id: &str) -> Result<()> {
    let mut billing = storage::get_tenant_billing(kv, tenant_id).await?;
    billing.replies_remaining += 1;
    billing.replies_used = (billing.replies_used - 1).max(0);
    storage::save_tenant_billing(kv, tenant_id, &billing).await
}

/// Grant reply credits to a tenant. Count must be positive.
pub async fn grant_replies(kv: &kv::KvStore, tenant_id: &str, count: i64) -> Result<()> {
    if count <= 0 {
        return Err(Error::from("Credit count must be positive"));
    }
    let mut billing = storage::get_tenant_billing(kv, tenant_id).await?;
    billing.replies_remaining = billing.replies_remaining.saturating_add(count);
    billing.replies_granted = billing.replies_granted.saturating_add(count);
    storage::save_tenant_billing(kv, tenant_id, &billing).await
}

#[cfg(test)]
mod tests {
    use crate::types::TenantBilling;

    #[test]
    fn has_credits_positive() {
        let b = TenantBilling {
            replies_remaining: 5,
            ..Default::default()
        };
        assert!(b.has_credits());
    }

    #[test]
    fn has_credits_zero() {
        let b = TenantBilling {
            replies_remaining: 0,
            ..Default::default()
        };
        assert!(!b.has_credits());
    }

    #[test]
    fn has_credits_negative() {
        let b = TenantBilling {
            replies_remaining: -3,
            ..Default::default()
        };
        assert!(!b.has_credits());
    }
}
