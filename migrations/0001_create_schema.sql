-- Single canonical schema, edited in place.

-- Tenants
CREATE TABLE IF NOT EXISTS tenants (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    facebook_id TEXT,
    plan TEXT DEFAULT 'paid',
    locale TEXT NOT NULL DEFAULT 'en-IN',
    currency TEXT NOT NULL DEFAULT 'INR',
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_tenants_email ON tenants(email);
CREATE INDEX IF NOT EXISTS idx_tenants_facebook ON tenants(facebook_id);

-- Unified message log (metadata-only; no message bodies stored).
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    channel TEXT NOT NULL,
    channel_account_id TEXT NOT NULL DEFAULT '',
    direction TEXT NOT NULL,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    action_taken TEXT,
    conversation_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_messages_tenant ON messages(tenant_id, created_at);
CREATE INDEX IF NOT EXISTS idx_messages_channel ON messages(channel, tenant_id, created_at);
CREATE INDEX IF NOT EXISTS idx_messages_channel_account ON messages(channel_account_id);
CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id, created_at);

-- Payment history
CREATE TABLE IF NOT EXISTS payments (
    id TEXT PRIMARY KEY,
    tenant_id TEXT,
    razorpay_payment_id TEXT,
    razorpay_order_id TEXT,
    amount INTEGER NOT NULL,
    credits INTEGER NOT NULL DEFAULT 0,
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
    replies_used INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
