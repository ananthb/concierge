//! `GET /demo/personas`: JSON list of Approved-only personas from the
//! D1 catalog, populated into the welcome page's chat picker on first
//! open. Stripped down to the four fields the picker needs (slug,
//! label, description, greeting); the demo-augmented prompt rides
//! along so the "View system prompt" panel matches what `/demo/chat`
//! actually sends. Builder rows carry their sample business fields
//! (name, type, city, hours) so the UI can render a "you're chatting
//! as a customer of …" card under the picker.

use serde::{Deserialize, Serialize};
use worker::*;

use crate::storage;

const CACHE_KEY_DEMO_PERSONAS: &str = "cache:demo:personas:v1";
const CACHE_TTL_SECONDS: u64 = 300;

#[derive(Serialize, Deserialize, Clone)]
struct DemoBusiness {
    name: String,
    business_type: String,
    city: String,
    hours: String,
    goal: String,
    goal_url: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct DemoPersonaPayload {
    slug: String,
    label: String,
    description: String,
    greeting: String,
    /// The exact middle (frame + persona prompt) sent to the model on
    /// `/demo/chat`. Rendered in the "View system prompt" panel.
    prompt: String,
    /// Sample business profile, present only for Builder personas. The
    /// system Concierge row has none. Its "business" is Concierge.
    #[serde(skip_serializing_if = "Option::is_none")]
    business: Option<DemoBusiness>,
}

#[derive(Serialize, Deserialize, Clone)]
struct DemoPersonasResponse {
    personas: Vec<DemoPersonaPayload>,
}

pub async fn handle_demo_personas(_req: Request, env: Env) -> Result<Response> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    // 1. Check KV cache
    if let Ok(Some(cached)) = kv
        .get(CACHE_KEY_DEMO_PERSONAS)
        .json::<DemoPersonasResponse>()
        .await
    {
        return json_response(cached);
    }

    // 2. Fetch approved archetypes
    let archetypes = storage::list_archetypes(&db, true)
        .await
        .unwrap_or_default();

    // 3. Generate fictional business details using LLM for each archetype
    // We do this in one batch prompt to save time and tokens.
    let mut personas = Vec::with_capacity(archetypes.len() + 1);

    // Add hardcoded Concierge first
    personas.push(DemoPersonaPayload {
        slug: "concierge".to_string(),
        label: "Concierge".to_string(),
        description: "Concierge talking about itself — what I am, channels, pricing, setup.".to_string(),
        greeting: "Hi! I'm Concierge. Ask me what I do, which channels I cover, how pricing works, or how to set me up.".to_string(),
        prompt: crate::prompt::compose_demo_middle(crate::prompt::CONCIERGE_PROMPT, "concierge"),
        business: None,
    });

    if !archetypes.is_empty() {
        let mut prompt_parts = Vec::new();
        for a in &archetypes {
            prompt_parts.push(format!("- {} ({})", a.label, a.description));
        }
        let system_prompt = "You are a creative business consultant. Generate fictional but realistic business details for each of the following persona archetypes. For each, provide: name, business_type, city, hours, and goal. Return ONLY a JSON array of objects, one for each archetype, in the exact same order. Do not include any other text.";
        let user_prompt = prompt_parts.join("\n");

        if let Ok(reply) = crate::ai::generate_chat_reply(
            &env,
            system_prompt,
            &vec![("user".to_string(), user_prompt)],
        )
        .await
        {
            // Try to parse the JSON array from the reply. The model might wrap it in ```json blocks.
            let clean_json = reply
                .trim_start_matches("```json")
                .trim_end_matches("```")
                .trim();
            if let Ok(businesses) = serde_json::from_str::<Vec<DemoBusiness>>(clean_json) {
                for (i, biz) in businesses.into_iter().enumerate() {
                    if let Some(a) = archetypes.get(i) {
                        let builder = crate::types::PersonaBuilder {
                            archetype_slug: a.slug.clone(),
                            biz_name: biz.name.clone(),
                            biz_type: biz.business_type.clone(),
                            city: biz.city.clone(),
                            hours: biz.hours.clone(),
                            goal: biz.goal.clone(),
                            goal_url: biz.goal_url.clone(),
                            ..Default::default()
                        };
                        let persona_middle = crate::personas::generate(&builder, &a.voice_prompt);
                        personas.push(DemoPersonaPayload {
                            slug: a.slug.clone(),
                            label: a.label.clone(),
                            description: a.description.clone(),
                            greeting: a.greeting.clone(),
                            prompt: crate::prompt::compose_demo_middle(&persona_middle, &a.slug),
                            business: Some(biz),
                        });
                    }
                }
            }
        }
    }

    let response = DemoPersonasResponse { personas };

    // 4. Cache in KV
    let _ = kv
        .put(CACHE_KEY_DEMO_PERSONAS, serde_json::to_string(&response)?)?
        .expiration_ttl(CACHE_TTL_SECONDS)
        .execute()
        .await;

    json_response(response)
}

fn json_response(data: DemoPersonasResponse) -> Result<Response> {
    let body = serde_json::to_string(&data)?;
    let mut headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Cache-Control", "no-store")?;
    Ok(Response::ok(body)?.with_headers(headers))
}
