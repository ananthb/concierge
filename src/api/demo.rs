//! `POST /api/demo/chat` — the landing page's interactive demo.
//!
//! A thin move of the old `/demo/chat` endpoint, which was already JSON:
//! the HTMX page talked to it with `fetch`, not a form post. The logic is
//! unchanged and lives in [`crate::handlers::demo_chat`]; only the path
//! moved under `/api/`.
//!
//! Notably public and rate-limited rather than session-gated: the whole
//! point is that a visitor can try it before signing up.

use worker::*;

pub async fn chat(req: Request, env: Env) -> Result<Response> {
    crate::handlers::handle_demo_chat(req, env).await
}
