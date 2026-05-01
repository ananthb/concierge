-- Single canonical schema. Merged from the original 0001-0004 progression
-- ahead of the approval-workflow deploy. The deploy workflow drops all
-- tables (and the d1_migrations bookkeeping table) before reapplying this
-- file, so we can write the final shape directly without ALTER chains.

-- Tenants
CREATE TABLE IF NOT EXISTS tenants (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    facebook_id TEXT,
    plan TEXT DEFAULT 'free',
    currency TEXT NOT NULL DEFAULT 'INR',
    locale TEXT NOT NULL DEFAULT 'en-IN',
    email_address_extras_purchased INTEGER NOT NULL DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_tenants_email ON tenants(email);
CREATE INDEX IF NOT EXISTS idx_tenants_facebook ON tenants(facebook_id);

-- WhatsApp message logging
CREATE TABLE IF NOT EXISTS whatsapp_messages (
    id TEXT PRIMARY KEY,
    whatsapp_account_id TEXT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('inbound', 'outbound')),
    from_number TEXT NOT NULL,
    to_number TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_whatsapp_messages_account
    ON whatsapp_messages(whatsapp_account_id, created_at);
CREATE INDEX IF NOT EXISTS idx_whatsapp_messages_tenant
    ON whatsapp_messages(tenant_id, created_at);

-- Lead form submissions
CREATE TABLE IF NOT EXISTS lead_form_submissions (
    id TEXT PRIMARY KEY,
    lead_form_id TEXT NOT NULL,
    phone_number TEXT NOT NULL,
    whatsapp_account_id TEXT NOT NULL,
    message_sent TEXT NOT NULL,
    reply_mode TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_lead_form_submissions_form
    ON lead_form_submissions(lead_form_id, created_at);
CREATE INDEX IF NOT EXISTS idx_lead_form_submissions_tenant
    ON lead_form_submissions(tenant_id, created_at);

-- Instagram messages
CREATE TABLE IF NOT EXISTS instagram_messages (
    id TEXT PRIMARY KEY,
    instagram_account_id TEXT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('inbound', 'outbound')),
    sender_id TEXT NOT NULL,
    recipient_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_instagram_messages_account
    ON instagram_messages(instagram_account_id, created_at);
CREATE INDEX IF NOT EXISTS idx_instagram_messages_tenant
    ON instagram_messages(tenant_id, created_at);

-- Unified message log (all channels)
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    channel TEXT NOT NULL,
    direction TEXT NOT NULL,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    channel_account_id TEXT NOT NULL DEFAULT '',
    action_taken TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_messages_tenant ON messages(tenant_id, created_at);
CREATE INDEX IF NOT EXISTS idx_messages_channel ON messages(channel, tenant_id, created_at);
CREATE INDEX IF NOT EXISTS idx_messages_channel_account ON messages(channel_account_id);

-- Payment history
CREATE TABLE IF NOT EXISTS payments (
    id TEXT PRIMARY KEY,
    tenant_id TEXT,
    razorpay_payment_id TEXT,
    razorpay_subscription_id TEXT,
    amount INTEGER NOT NULL,
    currency TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_payments_tenant ON payments(tenant_id, created_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_payments_razorpay_id ON payments(razorpay_payment_id);

-- Tenant billing (credit ledger)
CREATE TABLE IF NOT EXISTS tenant_billing (
    tenant_id TEXT PRIMARY KEY,
    credits_json TEXT NOT NULL DEFAULT '[]',
    free_month TEXT,
    replies_used INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Audit log
CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY,
    actor_email TEXT NOT NULL,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT,
    details TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Pending AI-draft approvals. One row per draft that's been queued instead
-- of sent: either the rule's policy is `Always`, or `Auto` and the risk
-- gate fired. The id matches the KV ConversationContext id, so the Discord
-- button handler and the web routes can join through a single token.
CREATE TABLE IF NOT EXISTS pending_approvals (
    id                  TEXT PRIMARY KEY,
    tenant_id           TEXT NOT NULL,
    channel             TEXT NOT NULL,
    channel_account_id  TEXT NOT NULL,
    rule_id             TEXT NOT NULL,
    rule_label          TEXT NOT NULL,
    sender              TEXT NOT NULL,
    sender_name         TEXT,
    inbound_preview     TEXT NOT NULL,
    draft               TEXT NOT NULL,
    queue_reason        TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'pending',
    created_at          TEXT NOT NULL DEFAULT (datetime('now')),
    decided_at          TEXT,
    decided_by          TEXT,
    edited              INTEGER NOT NULL DEFAULT 0,
    last_digest_at      TEXT
);

CREATE INDEX IF NOT EXISTS idx_pa_tenant_status
    ON pending_approvals(tenant_id, status, created_at);
CREATE INDEX IF NOT EXISTS idx_pa_status_created
    ON pending_approvals(status, created_at);

-- Global settings (pricing, feature flags, etc.)
CREATE TABLE IF NOT EXISTS global_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO global_settings (key, value) VALUES ('unit_price_millipaise', '10000');
INSERT OR IGNORE INTO global_settings (key, value) VALUES ('unit_price_millicents', '100');
-- Reply-email subscription: each pack of N addresses costs `address_price_*`
-- (in paise / cents) per recurring period.
INSERT OR IGNORE INTO global_settings (key, value) VALUES ('address_price_paise', '9900');
INSERT OR IGNORE INTO global_settings (key, value) VALUES ('address_price_cents', '100');
INSERT OR IGNORE INTO global_settings (key, value) VALUES ('email_pack_size', '5');
INSERT OR IGNORE INTO global_settings (key, value) VALUES ('free_monthly_credits', '100');

-- Scheduled credit grants. The scheduled-grants cron picks every row where
-- next_run_at <= now AND active = 1, grants `credits` to the targeted
-- tenants, then advances next_run_at by the cadence.
--
-- audience_kind = 'everyone'  → grant to every tenant in the tenants table.
-- audience_kind = 'emails'    → grant only to tenants whose email is in
--                               the JSON array `audience_emails`.
-- cadence is one of: daily, weekly_<dow>, monthly_first
--   weekly_<dow> uses lowercase 3-letter day codes: mon, tue, wed, thu, fri, sat, sun.
-- expires_in_days controls the granted-credit expiry (0 = never expires).
CREATE TABLE IF NOT EXISTS scheduled_grants (
    id TEXT PRIMARY KEY,
    cadence TEXT NOT NULL,
    audience_kind TEXT NOT NULL CHECK (audience_kind IN ('everyone', 'emails')),
    audience_emails TEXT NOT NULL DEFAULT '[]',
    credits INTEGER NOT NULL CHECK (credits > 0),
    expires_in_days INTEGER NOT NULL DEFAULT 0,
    last_run_at TEXT,
    next_run_at TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_sg_due
    ON scheduled_grants(active, next_run_at);
