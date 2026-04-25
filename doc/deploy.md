# Deployment Guide (Self-Hosting)

Concierge is designed to be easily forked and deployed to your own Cloudflare account using GitHub Actions.

## Self-hosting / Forking

1.  **Fork this repository** to your own GitHub account.
2.  **Create a D1 database** in the Cloudflare dashboard or via `wrangler d1 create concierge-worker`.
3.  **Create a KV namespace** via `wrangler kv namespace create KV`.
4.  **Configure GitHub Secrets** in your forked repository (Settings → Secrets and variables → Actions):
    *   `CLOUDFLARE_API_TOKEN`: A Cloudflare API token with `Workers`, `D1`, `KV`, and `Email Routing` permissions.
    *   `CLOUDFLARE_ACCOUNT_ID`: Your Cloudflare Account ID.
    *   `D1_DATABASE_ID`: The ID of your `concierge-worker` D1 database.
    *   `KV_NAMESPACE_ID`: The ID of your `KV` namespace.
    *   `RAZORPAY_KEY_ID` / `RAZORPAY_KEY_SECRET`: Razorpay API credentials for billing.
    *   `RAZORPAY_WEBHOOK_SECRET`: Webhook secret from Razorpay Dashboard.
    *   `CF_ACCESS_TEAM` / `CF_ACCESS_AUD`: Credentials for Cloudflare Access (protects `/manage`).
    *   `META_APP_ID` / `META_APP_SECRET`: Meta App credentials for WhatsApp/Instagram.
5.  **Push to `main`** to trigger the deployment workflow. GitHub Actions will automatically apply database migrations and deploy the worker.

## Post-deployment Setup
Once the worker is deployed, follow the detailed setup guides for each channel:
*   [WhatsApp Setup](whatsapp.html)
*   [Instagram Setup](instagram.html)
*   [Email Routing Setup](email-routing.html)
*   [Discord Setup](discord.html)
