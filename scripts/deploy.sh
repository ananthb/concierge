#!/usr/bin/env bash
# Apply D1 migrations and deploy the Worker without wiping Cloudflare
# Dashboard runtime variables (`--keep-vars`). Any plaintext variable
# present in the build environment is forwarded via `--var`; unset
# variables are omitted so they never overwrite existing runtime vars
# with an empty string.
set -euo pipefail

cd "$(dirname "$0")/.."

wrangler d1 migrations apply replybots --remote

args=()
for key in \
    PUBLIC_BASE_URL \
    META_APP_ID \
    WHATSAPP_SIGNUP_CONFIG_ID \
    RAZORPAY_KEY_ID \
    EMAIL_DOMAIN \
    WHATSAPP_WABA_ID
do
    val="${!key:-}"
    if [ -n "$val" ]; then
        args+=(--var "${key}:${val}")
    fi
done

exec wrangler deploy --keep-vars "${args[@]}"
