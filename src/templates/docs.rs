//! Public /docs page — technical reference for everything Concierge does.
//! No marketing voice; just the architecture, APIs, schemas, and limits.

use super::base::{base_html_with_meta, footer, public_nav_html, PageMeta};

pub fn docs_html() -> String {
    let nav = public_nav_html("docs");
    let foot = footer();

    let content = format!(
        r##"{nav}
<article class="legal">
  <h1>Technical reference</h1>
  <p class="muted">Concierge is a Cloudflare Worker (Rust → WebAssembly). All persistent state lives in Cloudflare D1 (metadata, payments) and KV (configs, sessions, in-flight buffers). No message content is stored at rest.</p>

  <h2>Inbound channels</h2>

  <h3>WhatsApp Business</h3>
  <ul>
    <li><strong>Transport:</strong> Meta Cloud API webhooks at <code>POST /webhook/whatsapp</code>.</li>
    <li><strong>Auth:</strong> WhatsApp Embedded Signup OAuth → tenant exchanges code for system-user token, scoped to one phone number ID.</li>
    <li><strong>Tenant lookup:</strong> reverse index <code>wa_phone:{{phone_number_id}}</code> → WhatsApp account id → tenant id.</li>
    <li><strong>Outbound:</strong> Meta Graph API <code>POST /{{phone_number_id}}/messages</code> with the system-user token.</li>
    <li><strong>Limits:</strong> no message-history endpoint, so post-hoc batching can't reconstruct text from older messages — see Reply Buffer below.</li>
  </ul>

  <h3>Instagram DMs</h3>
  <ul>
    <li><strong>Transport:</strong> Meta webhook events, same handler family as WhatsApp.</li>
    <li><strong>Auth:</strong> Facebook Login → finds the user's Pages → finds the IG business account on each page → stores per-page access token (AES-256-GCM encrypted with <code>ENCRYPTION_KEY</code>).</li>
    <li><strong>Tenant lookup:</strong> <code>ig_page:{{page_id}}</code> reverse index → IG account → tenant.</li>
    <li><strong>Outbound:</strong> Graph API <code>POST /me/messages</code> with the page token.</li>
  </ul>

  <h3>Discord</h3>
  <ul>
    <li><strong>Install:</strong> OAuth2 <code>scope=bot+applications.commands</code>, permission bitfield <code>76928</code> (SEND_MESSAGES | VIEW_CHANNEL | READ_MESSAGE_HISTORY | ADD_REACTIONS | MANAGE_MESSAGES). Callback at <code>/auth/discord/callback</code> records <code>guild_id → tenant_id</code> in KV.</li>
    <li><strong>Inbound transport:</strong> Application Webhook Events at <code>POST /discord/events</code>. <code>MESSAGE_CREATE</code> events drive AI auto-reply.</li>
    <li><strong>Triggers:</strong> per-tenant flags on <code>DiscordConfig</code> — <code>inbound_mentions</code> (reply when @-mentioned), <code>inbound_channel_ids[]</code> (reply to every message in these channels). DMs unsupported with the shared bot.</li>
    <li><strong>Interactions:</strong> <code>POST /discord/interactions</code> handles slash commands (<code>/status</code>, <code>/domains list</code>, <code>/rules list</code>) and buttons (Reply, Approve, Reject, Drop).</li>
    <li><strong>Signature verification:</strong> Ed25519 over <code>timestamp + body</code> using <code>DISCORD_PUBLIC_KEY</code>; same scheme for both endpoints.</li>
    <li><strong>Outbound:</strong> shared bot token (<code>DISCORD_BOT_TOKEN</code> env secret), POST to <code>/channels/{{id}}/messages</code> via the <code>botrelay</code> crate.</li>
  </ul>

  <h3>Email</h3>
  <ul>
    <li><strong>Transport:</strong> Cloudflare Email Routing — every <code>*.cncg.email</code> subdomain gets MX records pointed at the worker. Inbound mail invokes the worker's <code>email</code> event handler with the raw RFC 2822 bytes.</li>
    <li><strong>Tenant lookup:</strong> <code>email_domain:{{domain}}</code> KV reverse index.</li>
    <li><strong>Routing rules:</strong> per-domain ordered list in KV at <code>email_rules:{{tenant}}:{{domain}}</code>. Each rule has <code>MatchCriteria</code> (from, to, subject, body globs + has_attachment) and an <code>EmailAction</code> (drop, spam, forward_email, forward_discord, ai_reply).</li>
    <li><strong>Outbound:</strong> Cloudflare Email Service via the <code>EMAIL</code> binding's structured-message API (<code>{{from, to, subject, text, html, replyTo, headers}}</code>). Sender domain must be onboarded in the Email Service dashboard.</li>
    <li><strong>Reverse aliases:</strong> when forwarding, the From header is rewritten to a generated address on the tenant's domain so replies route back through Concierge. Mapping stored in <code>email_reverse:*</code> with 30-day TTL.</li>
    <li><strong>Loop detection:</strong> outbound messages carry <code>X-EmailProxy-Forwarded</code>; inbound messages with that header are rejected.</li>
  </ul>

  <h2>AI reply pipeline</h2>
  <ul>
    <li><strong>Inference binding:</strong> Cloudflare Workers AI <code>AI</code> binding. Default models — <code>llama-4-scout-17b-16e-instruct</code> for replies, <code>llama-3.1-8b-instruct-fast</code> for prompt-injection scanning. Configurable via <code>AI_MODEL</code> / <code>AI_FAST_MODEL</code> env vars.</li>
    <li><strong>Prompt construction:</strong> <code>PersonaConfig::to_system_prompt()</code> emits the tenant's voice/tone/never-do prefix; per-channel prompt overrides via <code>AutoReplyConfig.prompt</code>; per-rule prompt overrides via <code>EmailAction::AiReply.system_prompt</code>.</li>
    <li><strong>Injection scan:</strong> incoming bodies are truncated to 1000 chars, then a fast classifier checks for instruction-override patterns. Rejected messages skip the AI call and restore the credit.</li>
    <li><strong>Billing:</strong> only AI replies deduct a credit; static replies are free. Deduction happens before the AI call (optimistic), restored on any failure path. Free monthly grant of 100 credits per tenant.</li>
    <li><strong>Pricing:</strong> flat $0.02 / ₹2 per AI reply, no tiers. <code>UNIT_PRICE_PAISE = 200</code>, <code>UNIT_PRICE_CENTS = 2</code> in <code>src/billing/mod.rs</code>.</li>
  </ul>

  <h2>Reply buffer (Durable Object)</h2>
  <ul>
    <li><strong>Class:</strong> <code>ReplyBufferDO</code> in <code>src/durable_objects/reply_buffer.rs</code>; binding <code>REPLY_BUFFER</code>.</li>
    <li><strong>Keying:</strong> one DO instance per <code>{{tenant_id}}:{{channel}}:{{sender}}</code> conversation.</li>
    <li><strong>Sliding window:</strong> each push appends to a pending list and resets the alarm to <code>now + wait_seconds</code>. Bursts collapse into one alarm fire.</li>
    <li><strong>Drop-after-send:</strong> the alarm handler clears DO storage <em>before</em> calling the LLM. Bodies live in DO state for ≤ wait_seconds (5s default), then gone.</li>
    <li><strong>Bypass:</strong> <code>wait_seconds = 0</code> on the channel's <code>AutoReplyConfig</code> skips the buffer for instant replies.</li>
  </ul>

  <h2>Approval relay</h2>
  <ul>
    <li><strong>Discord:</strong> AI drafts post to the tenant's approval channel as embeds with Approve/Reject buttons. Button click triggers <code>/discord/interactions</code> → component handler → outbound send via the originating channel adapter.</li>
    <li><strong>Conversation context:</strong> stored in KV at <code>conv:{{id}}</code> with 7-day TTL, holds the Discord message id and origin channel/sender so the reply routes back correctly.</li>
    <li><strong>Email:</strong> approval-by-email digest sent at the tenant's configured cadence (default 15 min); links contain signed tokens for one-click approve/reject.</li>
  </ul>

  <h2>Lead capture forms</h2>
  <ul>
    <li><strong>Storage:</strong> <code>LeadCaptureForm</code> in KV at <code>lead_form:{{id}}</code>, indexed by tenant.</li>
    <li><strong>Rendering:</strong> <code>GET /lead/{{id}}/{{slug}}</code> serves an iframe-friendly HTML form; CSP and <code>allowed_origins</code> restrict where it embeds.</li>
    <li><strong>Submission:</strong> <code>POST</code> to the same path validates the phone number and triggers a WhatsApp message via the configured account, then logs to <code>lead_form_submissions</code> in D1.</li>
  </ul>

  <h2>Storage layout</h2>

  <h3>D1 tables</h3>
  <ul>
    <li><code>tenants</code> — id, email (UNIQUE), facebook_id, plan, currency.</li>
    <li><code>messages</code> — unified inbound/outbound metadata (channel, direction, sender, recipient, action_taken). No body content.</li>
    <li><code>whatsapp_messages</code>, <code>instagram_messages</code>, <code>email_messages</code>, <code>email_metrics</code>, <code>lead_form_submissions</code> — channel-specific logs.</li>
    <li><code>tenant_billing</code> — credit ledger as JSON (entries with optional expiry).</li>
    <li><code>payments</code> — Razorpay event log for compliance.</li>
    <li><code>audit_log</code> — management-action history.</li>
  </ul>

  <h3>KV keys</h3>
  <ul>
    <li><code>session:*</code>, <code>csrf:*</code> — auth cookies (TTL 7d).</li>
    <li><code>whatsapp:{{id}}</code>, <code>instagram:{{id}}</code>, <code>lead_form:{{id}}</code> — per-resource configs.</li>
    <li><code>tenant:{{tenant}}:whatsapp:{{id}}</code> etc. — per-tenant indexes (empty values; existence is the index).</li>
    <li><code>wa_phone:*</code>, <code>ig_page:*</code>, <code>email_domain:*</code> — webhook → tenant reverse indexes.</li>
    <li><code>email_domains:{{tenant}}</code>, <code>email_rules:{{tenant}}:{{domain}}</code>, <code>email_reverse:*</code> — email config + alias mapping.</li>
    <li><code>discord_guild:{{guild_id}}</code>, <code>discord_config:{{tenant}}</code> — guild ↔ tenant.</li>
    <li><code>onboarding:{{tenant}}</code> — wizard state.</li>
    <li><code>conv:{{id}}</code> — approval-relay conversation context (TTL 7d).</li>
  </ul>

  <h2>Auth</h2>
  <ul>
    <li><strong>Login:</strong> Google OAuth (<code>/auth/callback</code>) and Facebook Login (<code>/auth/facebook/callback</code>). Same tenant gets linked to both providers if their email matches.</li>
    <li><strong>Session:</strong> 7-day HttpOnly cookie; CSRF via double-submit cookie checked on every <code>POST/PUT/DELETE</code> under <code>/admin</code>.</li>
    <li><strong>Management panel:</strong> <code>/manage/*</code> protected by Cloudflare Access (verifies the <code>Cf-Access-Jwt-Assertion</code> header against the team's JWKS).</li>
  </ul>

  <h2>Outbound webhooks Concierge calls</h2>
  <ul>
    <li>Meta Graph API for WhatsApp + Instagram + Facebook Login.</li>
    <li>Discord REST API (<code>discord.com/api/v10</code>) for messages, channels, guild lookup.</li>
    <li>Cloudflare DNS API for provisioning MX + A/AAAA records on tenant subdomains.</li>
    <li>Razorpay API for orders, subscriptions, payment verification.</li>
    <li>Cloudflare Workers AI binding (no HTTP — direct binding call).</li>
  </ul>

  <h2>Limits and known constraints</h2>
  <ul>
    <li>Discord DM auto-reply is unsupported with the shared bot — incoming DMs hit the events endpoint with no <code>guild_id</code>, so we can't attribute them to a tenant.</li>
    <li>WhatsApp has no message-history API; the reply buffer relies on its own DO state to reconstruct bursts.</li>
    <li>Cloudflare Email Service requires sender domains to be onboarded in the dashboard before sends from them succeed; new tenant subdomains may need manual onboarding until that step is automated.</li>
    <li>No per-message body storage. If you need a conversation web-view, that's a future feature requiring a schema change and ToS update.</li>
  </ul>

  <h2>Discord developer-portal setup (one-time, per deployment)</h2>
  <p>Concierge uses one shared Discord application across all tenants. Configure it once when you first deploy.</p>
  <ol>
    <li><strong>Create the app.</strong> Go to <a href="https://discord.com/developers/applications" target="_blank" rel="noopener">discord.com/developers/applications</a> and create a new application. Name it whatever your product is called.</li>
    <li><strong>Bot user.</strong> Open the <em>Bot</em> tab → <em>Reset Token</em> and copy the token. Under <em>Privileged Gateway Intents</em>, enable <strong>Message Content Intent</strong> (required so <code>MESSAGE_CREATE</code> events carry the message body).</li>
    <li><strong>Identifiers.</strong> On the <em>General Information</em> tab, copy the <em>Application ID</em> and <em>Public Key</em>.</li>
    <li><strong>OAuth2 redirect.</strong> Open the <em>OAuth2</em> tab → <em>Redirects</em> → add <code>{{BASE_URL}}/auth/discord/callback</code>.</li>
    <li><strong>Interactions endpoint.</strong> On the <em>General Information</em> tab, set <em>Interactions Endpoint URL</em> to <code>{{BASE_URL}}/discord/interactions</code>. Discord pings the URL on save; the worker must already be deployed with the secrets below for the handshake to succeed.</li>
    <li><strong>Webhook events.</strong> On the <em>Webhooks</em> (or <em>Event Webhooks</em>) tab, set <em>Webhook Event URL</em> to <code>{{BASE_URL}}/discord/events</code> and subscribe to <code>message.create</code>. Same Ed25519 handshake as the interactions endpoint.</li>
    <li><strong>Worker secrets.</strong> Run:
      <pre class="mono"><code>wrangler secret put DISCORD_APPLICATION_ID
wrangler secret put DISCORD_PUBLIC_KEY
wrangler secret put DISCORD_BOT_TOKEN
wrangler secret put DISCORD_APP_ID  # alias used by some handlers</code></pre>
    </li>
    <li><strong>Slash commands (optional).</strong> Register <code>/status</code>, <code>/domains list</code>, <code>/rules list &lt;domain&gt;</code> globally via <code>POST /applications/{{app_id}}/commands</code>. The interactions handler is already wired to dispatch them.</li>
  </ol>
  <p class="muted">After this, every tenant just clicks <em>Install</em> on <code>/admin/discord</code> (or the wizard's Channels step) — the OAuth flow handles per-tenant guild attribution and channel picking.</p>

  <h2>Self-hosting</h2>
  <p>The repo at <a href="https://github.com/ananthb/concierge-worker" target="_blank" rel="noopener">github.com/ananthb/concierge-worker</a> is AGPL-3.0. Required Cloudflare bindings: D1, KV, AI, EMAIL (Email Routing + Email Service), DURABLE_OBJECTS. Required env secrets are listed in <code>wrangler.toml</code>'s comment block. The setup walkthrough lives at <a href="https://ananthb.github.io/concierge-worker/" target="_blank" rel="noopener">ananthb.github.io/concierge-worker</a>.</p>
</article>
{foot}"##,
        nav = nav,
        foot = foot,
    );

    base_html_with_meta(
        "Docs - Concierge",
        &content,
        &PageMeta {
            description: "Technical reference for Concierge — channels, AI pipeline, storage layout, auth, outbound APIs, and known limits.",
            og_title: "Concierge Docs",
            ..PageMeta::default()
        },
    )
}
