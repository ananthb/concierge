# Billing

`replybots` uses strict prepaid credits:

- **Static replies are free; 1 credit = 1 AI-generated reply.** When a static reply has an optional match rule, confident matches send the static reply for free while non-matching or uncertain messages fall back to AI generation (1 credit).
- **No subscriptions, no free trials** — accounts start with 0 credits and top up via Razorpay Orders.
- **No charge on held or failed replies** — if the risk gate withholds a draft or AI generation / delivery fails before sending, no credit is deducted.
- **Purchased credits do not expire**.

## Payment Flow

1. **Order creation** (`POST /api/billing/order`) — creates a Razorpay Order for the requested credit count (between `minCredits` and `maxCredits`) in the tenant's currency (`INR` or `USD`).
2. **Checkout & verification** — the Elm dashboard opens Razorpay Checkout (`public/boot.js`) and posts the payment signature to `/api/billing/verify`.
3. **Authoritative webhook** (`POST /webhook/razorpay`, `src/billing/webhook.rs`) — verifies `X-Razorpay-Signature` (HMAC-SHA256) and fails closed if `RAZORPAY_WEBHOOK_SECRET` is unset or empty. Credits are recorded idempotently in `payments` and `tenant_billing`.
