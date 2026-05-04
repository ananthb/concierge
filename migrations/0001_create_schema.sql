-- Single canonical schema. Edits to this file do NOT propagate to remote
-- D1 automatically — `wrangler d1 migrations apply` skips files it has
-- already run. To change a deployed schema, add a fresh `000N_*.sql`
-- migration with the deltas, or drop the relevant tables and re-execute
-- this file with `wrangler d1 execute --file`.
--
-- TODO: this single-migration shape is a development convenience so the
-- schema stays readable in one place. Before the first production
-- deploy, freeze this file and switch to additive `000N_*.sql` delta
-- migrations — and delete this comment block.

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
    -- Set the first time we observe a captured Razorpay payment for this
    -- tenant. The sign-up wizard charges a small refundable amount as an
    -- abuse-prevention check, and any other captured payment also flips
    -- this. Used to gate wizard "Finish".
    verified_at TEXT,
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

-- Unified message log (all channels).
--
-- `conversation_id` ties a row to a `Session.conversation_id` (KV).
-- AI flows always stamp it on outbound rows; inbound rows leave it
-- NULL today (the inbound is logged before we know whether it'll
-- enter an AI conversation). Canned-only flows leave it NULL too —
-- those threads don't run through the conversation/handoff machine.
-- Reconstructing a conversation from this table = filter by
-- conversation_id on outbounds, then join to nearby inbounds via
-- (sender, channel_account_id) and timestamp.
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    channel TEXT NOT NULL,
    direction TEXT NOT NULL,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    channel_account_id TEXT NOT NULL DEFAULT '',
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

-- Singleton bag of currency-agnostic settings. Per-currency amounts live
-- in `pricing_amount` below.
CREATE TABLE IF NOT EXISTS pricing_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    -- Reply-email subscription pack size — addresses granted per pack
    -- purchase. Currency-independent; the price lives in pricing_amount.
    email_pack_size INTEGER NOT NULL DEFAULT 5,
    updated_at TEXT DEFAULT (datetime('now'))
);
INSERT OR IGNORE INTO pricing_config (id) VALUES (1);

-- Per-(concept, currency) pricing.
--
-- `concept` is one of:
--   'unit_price_milli'    — per-AI-reply rate, in milli-minor units (1/1000
--                           paise / cent / yen / etc) so sub-minor prices fit.
--   'address_price'       — reply-email pack price per recurring period, in
--                           minor units of the currency.
--   'verification_amount' — sign-up verification charge, in minor units.
--
-- Adding a currency = INSERT three rows here; no schema change needed.
-- Currency codes are ISO 4217 (e.g. INR, USD, EUR, JPY, KWD); we use
-- rusty_money's metadata to look up symbols + minor-unit exponents.
CREATE TABLE IF NOT EXISTS pricing_amount (
    concept TEXT NOT NULL,
    currency_code TEXT NOT NULL,
    amount INTEGER NOT NULL,
    PRIMARY KEY (concept, currency_code)
);

INSERT OR IGNORE INTO pricing_amount (concept, currency_code, amount) VALUES
    ('unit_price_milli',    'INR', 10000),  -- 10000 milli-paise = ₹0.10/reply
    ('unit_price_milli',    'USD', 100),    -- 100 milli-cents = $0.001/reply
    ('address_price',       'INR', 9900),   -- ₹99/pack/month
    ('address_price',       'USD', 100),    -- $1/pack/month
    ('verification_amount', 'INR', 100),    -- ₹1
    ('verification_amount', 'USD', 100);    -- $1

-- Scheduled credit grants. The scheduled-grants cron picks every row where
-- next_run_at <= now AND active = 1, grants `credits` to every tenant,
-- then advances next_run_at by the cadence.
--
-- cadence is one of: daily, weekly_<dow>, monthly_first
--   weekly_<dow> uses lowercase 3-letter day codes: mon, tue, wed, thu, fri, sat, sun.
-- expires_in_days controls the granted-credit expiry (0 = never expires).
CREATE TABLE IF NOT EXISTS scheduled_grants (
    id TEXT PRIMARY KEY,
    cadence TEXT NOT NULL,
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

-- Archetype catalog. Curated by management at /manage/archetypes, listed by
-- the public demo's persona picker, and referenced by a tenant's persona
-- config.
--
-- Every catalog edit resets `safety_status` to 'draft' and enqueues a
-- SafetyJob; Approved is the only state the demo and onboarding will read.
CREATE TABLE IF NOT EXISTS archetypes (
    slug                  TEXT PRIMARY KEY,
    label                 TEXT NOT NULL,
    description           TEXT NOT NULL,
    voice_prompt          TEXT NOT NULL,
    greeting              TEXT NOT NULL,
    default_rules_json    TEXT NOT NULL,
    catch_phrases_json    TEXT NOT NULL DEFAULT '[]',
    off_topics_json       TEXT NOT NULL DEFAULT '[]',
    never                 TEXT NOT NULL DEFAULT '',
    handoff_conditions_json TEXT NOT NULL DEFAULT '[]',
    safety_status         TEXT NOT NULL DEFAULT 'draft'
                          CHECK (safety_status IN ('draft','approved','rejected')),
    safety_checked_at     TEXT,
    safety_vague_reason   TEXT,
    created_at            TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at            TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_archetypes_status ON archetypes(safety_status);

-- Seed data. The four base archetypes ship Approved so the demo and
-- onboarding work the moment the migration runs. Management edits will
-- drive new rows through the classifier.
INSERT OR REPLACE INTO archetypes (slug, label, description, voice_prompt, greeting, default_rules_json, catch_phrases_json, off_topics_json, never, handoff_conditions_json, safety_status, safety_checked_at)
VALUES
    ('friendly',
     'Friendly',
     'A warm, kind voice. Speak like a shopkeeper who has known the customer for years.',
     'Voice: warm, kind, conversational. Speak like a shopkeeper who has known the customer for years. Confirm you would love to help, ask one clarifying question if you need it, let the customer know a human will follow up where needed.',
     'Hi there! How can we help you today?',
     '[{"id":"pricing","label":"Pricing questions","matcher":{"kind":"prompt","description":"asks about price, cost, or how much something is","embedding":[],"embedding_model":"","threshold":0.75},"response":{"kind":"prompt","text":"Confirm we''d love to help, ask what they have in mind, and let them know the owner will follow up with a quote."},"approval":"auto"},{"id":"after_hours","label":"After-hours messages","matcher":{"kind":"keyword","keywords":["after hours","closed","still open"]},"response":{"kind":"canned","text":"Thanks for reaching out — we''re closed right now but we''ll get back to you first thing."},"approval":"auto"}]',
     '[]', '[]', '', '[]',
     'approved', datetime('now')),
    ('professional',
     'Professional',
     'Concise and businesslike. Greet briefly, confirm what is possible.',
     'Voice: concise and professional. Greet briefly, confirm what is possible, ask for the missing detail. Defer firm commitments to a human follow-up.',
     'Thanks for reaching out. How can we help you today?',
     '[{"id":"pricing","label":"Pricing questions","matcher":{"kind":"prompt","description":"asks about price, cost, or how much something is","embedding":[],"embedding_model":"","threshold":0.75},"response":{"kind":"prompt","text":"Acknowledge the question, ask for the missing detail (what they need, by when), and confirm a human will respond with a price."},"approval":"auto"},{"id":"after_hours","label":"After-hours messages","matcher":{"kind":"keyword","keywords":["after hours","closed","still open"]},"response":{"kind":"canned","text":"We''re outside business hours; we''ll respond when we''re back."},"approval":"auto"}]',
     '[]', '[]', '', '[]',
     'approved', datetime('now')),
    ('playful',
     'Playful',
     'Upbeat and light. Light use of emoji when it fits naturally.',
     'Voice: playful and upbeat. Light use of emoji when it fits naturally. Stay warm without being cute.',
     'hi 👋 what can we do for u today? ✨',
     '[{"id":"pricing","label":"Pricing questions","matcher":{"kind":"prompt","description":"asks about price, cost, or how much something is","embedding":[],"embedding_model":"","threshold":0.75},"response":{"kind":"prompt","text":"Stay upbeat, ask what they''re after, and say someone will come back with the number soon."},"approval":"auto"},{"id":"after_hours","label":"After-hours messages","matcher":{"kind":"keyword","keywords":["after hours","closed","still open"]},"response":{"kind":"canned","text":"Catching some Zzz right now 💤 — we''ll write back when we''re up!"},"approval":"auto"}]',
     '[]', '[]', '', '[]',
     'approved', datetime('now')),
    ('formal',
     'Formal',
     'Polite and formal. Address the customer respectfully.',
     'Voice: polite and formal. Address the customer respectfully. Stay measured and considered; avoid casualness.',
     'Good day. How may we assist you today?',
     '[{"id":"pricing","label":"Pricing questions","matcher":{"kind":"prompt","description":"asks about price, cost, or how much something is","embedding":[],"embedding_model":"","threshold":0.75},"response":{"kind":"prompt","text":"Acknowledge the inquiry politely, ask for the relevant detail, and indicate that a member of the team will respond with the price."},"approval":"auto"},{"id":"after_hours","label":"After-hours messages","matcher":{"kind":"keyword","keywords":["after hours","closed","still open"]},"response":{"kind":"canned","text":"Thank you for your message. We are currently outside business hours and will respond at our earliest opportunity."},"approval":"auto"}]',
     '[]', '[]', '', '[]',
     'approved', datetime('now'));
