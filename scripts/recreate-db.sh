#!/usr/bin/env bash
# Drop every table and re-execute the canonical schema.
#
# BREAK-GLASS ONLY. The normal path is `POST /manage/reseed`, which does
# the same thing from inside the Worker using its own DB binding, so a
# `git push` is the whole deploy and nobody needs Cloudflare credentials.
# See src/management/reseed.rs. Use this script when the Worker cannot
# boot — a schema change that breaks startup is exactly when the endpoint
# is unreachable.
#
# migrations/0001_create_schema.sql is edited in place rather than
# extended with deltas. `wrangler d1 migrations apply` will not help: it
# skips a migration it has already run, and every CREATE is
# `IF NOT EXISTS`, so re-running it against a live database is a no-op
# that leaves dropped columns in place. The tables have to go first.
#
# DESTRUCTIVE: every row in the target database is deleted. Safe only
# while there is no data worth keeping. Requires an authenticated
# wrangler (`wrangler login`, or CLOUDFLARE_API_TOKEN in the env).
#
#   ./scripts/recreate-db.sh --local    # .wrangler/state, the dev copy
#   ./scripts/recreate-db.sh --remote   # the deployed database
set -euo pipefail

TARGET="${1:-}"
case "$TARGET" in
    --local | --remote) ;;
    *)
        echo "usage: $0 --local | --remote" >&2
        exit 2
        ;;
esac

DB_NAME="concierge"
SCHEMA="$(dirname "$0")/../migrations/0001_create_schema.sql"

# d1_migrations is wrangler's own bookkeeping: drop it too, or a later
# `migrations apply` believes 0001 has already run.
TABLES=(
    archetypes
    audit_log
    instagram_messages
    messages
    payments
    pending_approvals
    pricing_amount
    pricing_config
    tenant_billing
    tenants
    whatsapp_messages
    d1_migrations
)

if [ "$TARGET" = "--remote" ]; then
    echo "About to DROP every table in the deployed '$DB_NAME' database."
    printf 'Type the database name to confirm: '
    read -r reply
    if [ "$reply" != "$DB_NAME" ]; then
        echo "Aborted." >&2
        exit 1
    fi
fi

drops=""
for t in "${TABLES[@]}"; do
    drops="${drops}DROP TABLE IF EXISTS ${t}; "
done

echo "Dropping ${#TABLES[@]} tables ($TARGET)..."
wrangler d1 execute "$DB_NAME" "$TARGET" --yes --command "$drops"

echo "Re-executing $SCHEMA ..."
wrangler d1 execute "$DB_NAME" "$TARGET" --yes --file "$SCHEMA"

echo
echo "Done. Verifying the dropped columns are gone:"
wrangler d1 execute "$DB_NAME" "$TARGET" --yes \
    --command "SELECT name FROM pragma_table_info('tenants') WHERE name = 'email_address_extras_purchased' UNION ALL SELECT name FROM pragma_table_info('pricing_config') WHERE name = 'email_pack_size' UNION ALL SELECT concept FROM pricing_amount WHERE concept = 'address_price';"
echo "(an empty result above is the pass condition)"
