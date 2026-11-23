//! Static landing-page demo personas and `POST /api/demo/chat`.

use serde::Serialize;
use worker::*;

use crate::prompt::{BotPersona, BOT_PERSONAS};

pub const DEMO_MAX_USER_TURNS: u32 = 6;
pub const DEMO_IDLE_TIMEOUT_SECS: u32 = 300;

#[derive(Serialize, Clone, Debug)]
pub struct DemoPersonaView {
    pub id: &'static str,
    pub label: &'static str,
    pub tag: &'static str,
    pub business_name: &'static str,
    pub prompt: &'static str,
    pub opener: &'static str,
}

impl From<&BotPersona> for DemoPersonaView {
    fn from(p: &BotPersona) -> Self {
        Self {
            id: p.id,
            label: p.label,
            tag: p.tag,
            business_name: p.business_name,
            prompt: p.prompt,
            opener: p.opener,
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct DemoView {
    pub enabled: bool,
    pub max_user_turns: u32,
    pub idle_timeout_secs: u32,
    pub personas: Vec<DemoPersonaView>,
}

impl Default for DemoView {
    fn default() -> Self {
        Self {
            enabled: true,
            max_user_turns: DEMO_MAX_USER_TURNS,
            idle_timeout_secs: DEMO_IDLE_TIMEOUT_SECS,
            personas: BOT_PERSONAS.iter().map(DemoPersonaView::from).collect(),
        }
    }
}

pub async fn chat(req: Request, env: Env) -> Result<Response> {
    crate::handlers::handle_demo_chat(req, env).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_view_exports_four_personas() {
        let view = DemoView::default();
        assert!(view.enabled);
        assert_eq!(view.max_user_turns, DEMO_MAX_USER_TURNS);
        assert_eq!(view.idle_timeout_secs, DEMO_IDLE_TIMEOUT_SECS);
        assert_eq!(view.personas.len(), 4);
        assert_eq!(view.personas[0].id, "front-desk");
        assert_eq!(view.personas[1].id, "orders");
        assert_eq!(view.personas[2].id, "after-hours");
        assert_eq!(view.personas[3].id, "concise");
    }
}
