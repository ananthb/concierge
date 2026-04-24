-- Tenants
CREATE TABLE IF NOT EXISTS tenants (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    facebook_id TEXT,
    plan TEXT DEFAULT 'free',
    currency TEXT NOT NULL DEFAULT 'INR',
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

-- Email message log
CREATE TABLE IF NOT EXISTS email_messages (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    domain TEXT NOT NULL,
    rule_id TEXT,
    direction TEXT NOT NULL,
    from_email TEXT NOT NULL,
    to_email TEXT NOT NULL,
    action_taken TEXT NOT NULL,
    error_msg TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_email_msg_tenant ON email_messages(tenant_id, created_at);
CREATE INDEX IF NOT EXISTS idx_email_msg_domain ON email_messages(domain, created_at);

-- Email metrics counters
CREATE TABLE IF NOT EXISTS email_metrics (
    domain TEXT NOT NULL,
    rule_id TEXT,
    date TEXT NOT NULL,
    action_type TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    tenant_id TEXT NOT NULL,
    UNIQUE(domain, rule_id, date, action_type)
);
CREATE INDEX IF NOT EXISTS idx_email_metrics_domain_date ON email_metrics(domain, date);

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
