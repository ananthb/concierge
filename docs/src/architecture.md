# Architecture

`replybots` compiles to a single WebAssembly Worker (`src/lib.rs`) paired with static assets (`public/`) served from Cloudflare's edge.

## D1-Only State

All persistent state is stored in Cloudflare D1 (`migrations/0001_create_schema.sql`):

- **Single-database storage on Cloudflare D1** — burst debouncing, conversation sessions, rate limits, and billing ledgers all run on atomic D1 queries.
- **Metadata-only `messages` table** — stores `channel`, `direction`, `sender`, `recipient`, `action_taken`, `conversation_id`, and `created_at`, never message bodies.

## Inbound Reply Pipeline

1. **Webhook verification & dispatch** (`src/handlers/webhook.rs`) — verifies `X-Hub-Signature-256` with `META_APP_SECRET` and parses inbound WhatsApp events.
2. **Burst debounce (`pending_inbound`)** — buffers rapid consecutive customer messages for the configured `wait_seconds` (0–30s) so multiple short texts receive a single coherent reply.
3. **Prompt-injection screening** (`src/ai.rs`) — runs `@cf/meta/prompt-guard-2-86m` on inbound text.
4. **Static rule matching & reply generation** (`src/ai.rs`) — if any optional static `rules` (`{ match_rule, reply_text }`, up to 10) are configured, `@cf/meta/llama-3.1-8b-instruct-fast` selects the best matching rule (`matched_index` with `confidence >= 0.85`). Confident matches send that rule's `reply_text` verbatim; uncertain or non-matching messages (or configs with no rules) fall back to AI generation via `@cf/meta/llama-4-scout-17b-16e-instruct` with the single built-in core persona, custom `bot_prompt`, and recent conversation context (`conversation_sessions`). Every reply (static-rule or AI-generated) costs 1 credit.
5. **Safety & risk gate** (`src/safety.rs`, `src/risk.rs`) — vets output via `@cf/meta/llama-guard-3-8b` and `risk::signal` checks (unauthorized prices, commitments, or `[[HANDOFF]]` markers).
6. **Handoff & escalation** (`src/escalations.rs`) — when handed off, sends a holding reply, puts the thread into `handoff_cooldown_mins`, and optionally sends an alert email via Cloudflare `send_email`.

## Extending Channels

Outbound delivery and channel metadata are abstracted behind the `Channel` trait (`src/channel/`), allowing additional channels (such as Discord or Telegram) to plug into the same pipeline and D1 tables.
