//! `GET /demo/personas` — JSON list of Approved-only personas from the
//! D1 catalog, populated into the welcome page's chat picker on first
//! open. Stripped down to the four fields the picker needs (slug,
//! label, description, greeting); the full prompt rides along too so
//! the "View system prompt" panel doesn't need a second roundtrip.

use serde::Serialize;
use worker::*;

use crate::storage;

#[derive(Serialize)]
struct DemoPersonaPayload<'a> {
    slug: &'a str,
    label: &'a str,
    description: &'a str,
    greeting: &'a str,
    prompt: String,
}

#[derive(Serialize)]
struct DemoPersonasResponse<'a> {
    personas: Vec<DemoPersonaPayload<'a>>,
}

pub async fn handle_demo_personas(_req: Request, env: Env) -> Result<Response> {
    // Failure modes are tolerated quietly: dev environments without the
    // migration applied (or with a transient D1 hiccup) return an empty
    // list so the welcome page's chat picker shows "no personas yet"
    // instead of a network error in the console.
    let rows = match env.d1("DB") {
        Ok(db) => storage::list_personas(&db, true).await.unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    let payload: Vec<DemoPersonaPayload> = rows
        .iter()
        .map(|r| DemoPersonaPayload {
            slug: &r.slug,
            label: &r.label,
            description: &r.description,
            greeting: &r.greeting,
            prompt: r.source.active_prompt(),
        })
        .collect();

    let body = serde_json::to_string(&DemoPersonasResponse { personas: payload })
        .map_err(|e| Error::from(format!("serialise demo personas: {e}")))?;
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Cache-Control", "no-store")?;
    Ok(Response::ok(body)?.with_headers(headers))
}
