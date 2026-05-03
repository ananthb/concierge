//! `GET /demo/personas`: JSON list of Approved-only personas from the
//! D1 catalog, populated into the welcome page's chat picker on first
//! open. Stripped down to the four fields the picker needs (slug,
//! label, description, greeting); the demo-augmented prompt rides
//! along so the "View system prompt" panel matches what `/demo/chat`
//! actually sends. Builder rows carry their sample business fields
//! (name, type, city, hours) so the UI can render a "you're chatting
//! as a customer of …" card under the picker.

use serde::Serialize;
use worker::*;

use crate::storage;
use crate::types::PersonaSource;

#[derive(Serialize)]
struct DemoBusiness<'a> {
    name: &'a str,
    business_type: &'a str,
    city: &'a str,
    hours: &'a str,
    goal: &'a str,
    goal_url: &'a str,
}

#[derive(Serialize)]
struct DemoPersonaPayload<'a> {
    slug: &'a str,
    label: &'a str,
    description: &'a str,
    greeting: &'a str,
    is_system: bool,
    /// The exact middle (frame + persona prompt) sent to the model on
    /// `/demo/chat`. Rendered in the "View system prompt" panel.
    prompt: String,
    /// Sample business profile, present only for Builder personas. The
    /// system Concierge row has none. Its "business" is Concierge.
    #[serde(skip_serializing_if = "Option::is_none")]
    business: Option<DemoBusiness<'a>>,
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
        .map(|r| {
            let persona_middle = r.source.active_prompt();
            let prompt = crate::prompt::compose_demo_middle(&persona_middle, r.is_system);
            let business = match &r.source {
                PersonaSource::Builder(b) => Some(DemoBusiness {
                    name: &b.biz_name,
                    business_type: &b.biz_type,
                    city: &b.city,
                    hours: &b.hours,
                    goal: &b.goal,
                    goal_url: &b.goal_url,
                }),
                PersonaSource::Custom(_) => None,
            };
            DemoPersonaPayload {
                slug: &r.slug,
                label: &r.label,
                description: &r.description,
                greeting: &r.greeting,
                is_system: r.is_system,
                prompt,
                business,
            }
        })
        .collect();

    let body = serde_json::to_string(&DemoPersonasResponse { personas: payload })
        .map_err(|e| Error::from(format!("serialise demo personas: {e}")))?;
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Cache-Control", "no-store")?;
    Ok(Response::ok(body)?.with_headers(headers))
}
