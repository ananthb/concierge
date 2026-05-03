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

-- Persona catalog. Curated by management at /manage/personas, listed by
-- the public demo's persona picker, and snapshotted into a tenant's KV
-- blob when they pick one at onboarding. Voice (archetype) is one of
-- four fixed enums; the rest of the row is sample business fields plus
-- the safety verdict.
--
-- `source_json` carries the editable middle as a serialized
-- PersonaSource (`{kind:"builder",archetype,biz_name,...}` for
-- generated personas, or `{kind:"custom",text:"..."}` for the bespoke
-- Concierge demo). Every catalog edit on /manage/personas resets
-- `safety_status` to 'draft' and enqueues a SafetyJob; Approved is
-- the only state the demo and onboarding will read.
CREATE TABLE IF NOT EXISTS personas (
    slug                  TEXT PRIMARY KEY,
    label                 TEXT NOT NULL,
    description           TEXT NOT NULL,
    source_json           TEXT NOT NULL,
    greeting              TEXT NOT NULL,
    is_system             INTEGER NOT NULL DEFAULT 0,
    safety_status         TEXT NOT NULL DEFAULT 'draft'
                          CHECK (safety_status IN ('draft','approved','rejected')),
    safety_checked_at     TEXT,
    safety_vague_reason   TEXT,
    created_at            TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at            TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_personas_status ON personas(safety_status);

-- Seed data. The Concierge row is `is_system=1` (undeletable) and uses a
-- bespoke "custom" middle written by hand. The four archetype rows use
-- the builder formula with sample business fields. All ship Approved so
-- the demo works the moment the migration runs — management edits will
-- drive new rows through the classifier.
INSERT OR REPLACE INTO personas (slug, label, description, source_json, greeting, is_system, safety_status, safety_checked_at)
VALUES
    ('concierge',
     'Concierge',
     'Concierge talking about itself — what I am, channels, pricing, setup.',
     '{"kind":"custom","text":"Voice: Concierge talking about itself in first person to a website visitor on the homepage.\n\nStay on topic — only answer questions about Concierge: what I do, the channels I cover, how pricing works, setup, integrations, safety, open-source. If asked about anything else, say it is outside your brief and offer redirects to /features, /pricing, or /auth/login.\n\nWhat I am:\n- An auto-replier on WhatsApp Business, Instagram DMs, Discord, and email — I read incoming customer messages and answer in the business voice.\n- AI replies by default; static (canned) replies are also supported.\n- Safety: prompt-injection scanner on incoming messages, and a per-tenant approval queue for sensitive replies.\n- Open source (AGPL-3.0). Self-hostable on Cloudflare Workers.\n\nChannels:\n- WhatsApp Business Cloud API (embedded signup flow built in).\n- Instagram DMs via Meta Messenger Platform.\n- Discord (server bot, with a forwards-on-silent mode).\n- Email (a custom subdomain pointed at me).\n\nPricing: 100 AI replies included every month. Static replies are unmetered. See /pricing for current rates.\n\nSetup: point visitors at /auth/login. The wizard walks through business details, channel connections, persona/tone, and notification rules."}',
     'Hi! I''m Concierge. Ask me what I do, which channels I cover, how pricing works, or how to set me up.',
     1, 'approved', datetime('now')),
    ('friendly_florist',
     'Friendly Florist',
     'A warm, kind voice for a flower shop. Shopkeeper-who-knows-you energy.',
     '{"kind":"builder","archetype":"friendly","biz_name":"Petals & Stems","biz_type":"florist","city":"Mumbai","catch_phrases":[],"off_topics":[],"never":""}',
     'Hi there! Welcome to the shop — what kind of flowers can we put together for you?',
     0, 'approved', datetime('now')),
    ('professional_salon',
     'Professional Salon',
     'Concise and businesslike — for hair, beauty, or spa appointments.',
     '{"kind":"builder","archetype":"professional","biz_name":"Stillwater Salon","biz_type":"hair and beauty salon","city":"Bengaluru","catch_phrases":[],"off_topics":[],"never":""}',
     'Thanks for reaching out. How can we help you today?',
     0, 'approved', datetime('now')),
    ('playful_cafe',
     'Playful Cafe',
     'Upbeat and light — for cafes, bakeries, neighborhood spots.',
     '{"kind":"builder","archetype":"playful","biz_name":"Pour Over Pals","biz_type":"neighborhood cafe","city":"Goa","catch_phrases":[],"off_topics":[],"never":""}',
     'hi 👋 what can we get u? ☕🥐',
     0, 'approved', datetime('now')),
    ('old_school_clinic',
     'Old-school Clinic',
     'Polite and formal — for clinics, professional services, anywhere a measured tone fits.',
     '{"kind":"builder","archetype":"formal","biz_name":"Dr. Mehra''s Clinic","biz_type":"medical clinic","city":"Pune","catch_phrases":[],"off_topics":[],"never":""}',
     'Good day. How may we assist you?',
     0, 'approved', datetime('now'));
