//! `/admin/persona` — render the three-mode (Preset/Builder/Custom)
//! persona editor with a live prompt preview and a safety badge.
//!
//! The page is a single Alpine `x-data` form; the active mode swaps the
//! visible fields. Submission posts to the persona admin handler which
//! decides which source variant to construct.

use crate::helpers::html_escape;
use crate::personas;
use crate::types::{PersonaConfig, PersonaPreset, PersonaSafetyStatus, PersonaSource};

use super::base::{app_shell, base_html};

pub fn persona_admin_html(persona: &PersonaConfig, base_url: &str) -> String {
    // Active mode + a shadow copy of every field so switching modes doesn't
    // lose user input.
    let (active_mode, active_preset_slug, builder, custom_prompt) = match &persona.source {
        PersonaSource::Preset(p) => (
            "preset",
            p.slug(),
            crate::types::PersonaBuilder::default(),
            String::new(),
        ),
        PersonaSource::Builder(b) => ("builder", "", b.clone(), String::new()),
        PersonaSource::Custom(s) => (
            "custom",
            "",
            crate::types::PersonaBuilder::default(),
            s.clone(),
        ),
    };

    let preset_options: String = PersonaPreset::ALL
        .iter()
        .map(|p| {
            let slug = p.slug();
            let label = p.label();
            let desc = p.description();
            let checked = if slug == active_preset_slug {
                " checked"
            } else {
                ""
            };
            format!(
                r#"<label class="card p-14" style="cursor:pointer;display:block">
  <div class="row gap-10" style="align-items:flex-start">
    <input type="radio" name="preset_id" value="{slug}" x-model="presetId"{checked} style="margin-top:4px">
    <div class="flex-1">
      <div class="eyebrow mb-2">{label}</div>
      <p class="m-0 fs-13 muted">{desc}</p>
    </div>
  </div>
</label>"#,
                slug = html_escape(slug),
                label = html_escape(label),
                desc = html_escape(desc),
            )
        })
        .collect();

    let initial_preview = match &persona.source {
        PersonaSource::Preset(p) => p.prompt().to_string(),
        PersonaSource::Builder(b) => personas::generate(b),
        PersonaSource::Custom(s) => s.clone(),
    };

    let safety_badge = render_safety_badge(persona);

    let body = format!(
        r##"<div class="page-pad" x-data="{x_data}" hx-ext="json-enc">
  <p><a href="{base_url}/admin" class="btn ghost sm">&larr; Dashboard</a></p>
  <h1 class="display-sm m-0 mb-4">Persona</h1>
  <p class="muted mb-16">The persona is your AI assistant's voice. Every AI-generated reply uses this prompt as its system prompt.</p>

  {safety_badge}

  <div class="card p-22 mb-16">
    <div class="eyebrow mb-12">Mode</div>
    <div class="row gap-8 mb-16" style="flex-wrap:wrap">
      <label class="row gap-6"><input type="radio" name="mode" value="preset" x-model="mode"> Preset</label>
      <label class="row gap-6"><input type="radio" name="mode" value="builder" x-model="mode"> Builder</label>
      <label class="row gap-6"><input type="radio" name="mode" value="custom" x-model="mode"> Custom prompt</label>
    </div>

    <form hx-post="{base_url}/admin/persona" hx-target="body" hx-swap="innerHTML">
      <input type="hidden" name="mode" :value="mode">

      <!-- PRESET -->
      <div x-show="mode === 'preset'" x-cloak>
        <p class="muted fs-13 mb-12">Pick one of the curated personas. The prompt and a starter set of reply rules will be applied.</p>
        <div style="display:grid;gap:10px;grid-template-columns:1fr 1fr">
          {preset_options}
        </div>
        <input type="hidden" name="preset_id" :value="presetId">
      </div>

      <!-- BUILDER -->
      <div x-show="mode === 'builder'" x-cloak>
        <p class="muted fs-13 mb-12">Fill in the fields and we'll compose the prompt for you. Switch to Custom mode if you want to write the whole thing yourself.</p>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:12px">
          <div>
            <label class="eyebrow lbl">Type of business</label>
            <input class="input" name="biz_type" x-model="builder.biz_type" placeholder="florist, hair salon, cafe...">
          </div>
          <div>
            <label class="eyebrow lbl">City (optional)</label>
            <input class="input" name="city" x-model="builder.city" placeholder="Chennai, Berlin...">
          </div>
          <div>
            <label class="eyebrow lbl">Tone</label>
            <input class="input" name="tone" x-model="builder.tone" placeholder="warm and friendly, concise and professional...">
          </div>
          <div>
            <label class="eyebrow lbl">Never (one boundary)</label>
            <input class="input" name="never" x-model="builder.never" placeholder="quote prices, promise dates...">
          </div>
        </div>
        <div class="mt-12">
          <label class="eyebrow lbl">Catch-phrases (one per line, max 5)</label>
          <textarea class="textarea" name="catch_phrases" x-model="builder.catch_phrases" rows="3" placeholder="One catch-phrase per line"></textarea>
        </div>
        <div class="mt-12">
          <label class="eyebrow lbl">Off-topic subjects (one per line, max 10)</label>
          <textarea class="textarea" name="off_topics" x-model="builder.off_topics" rows="3" placeholder="politics, medical advice, refunds..."></textarea>
        </div>
      </div>

      <!-- CUSTOM -->
      <div x-show="mode === 'custom'" x-cloak>
        <p class="muted fs-13 mb-12">Write the entire system prompt yourself. Up to 2000 characters.</p>
        <textarea class="textarea mono" name="custom_prompt" x-model="customPrompt" rows="12" maxlength="2000" placeholder="You are a helpful assistant for..."></textarea>
        <p class="muted fs-12 mt-4"><span x-text="customPrompt.length"></span> / 2000</p>
      </div>

      <div class="row gap-8 mt-16" style="justify-content:flex-end">
        <button type="submit" class="btn primary">Save persona</button>
      </div>
    </form>
  </div>

  <div class="card p-14" style="background:var(--ink);color:var(--cream);border-color:var(--ink);border-radius:var(--r-sm)">
    <div class="mono fs-10 mb-6" style="letter-spacing:.18em;color:var(--accent-soft)">PROMPT PREVIEW</div>
    <div id="prompt-preview">
      <pre class="mono m-0 fs-12" style="white-space:pre-wrap;color:var(--cream);line-height:1.5">{initial_preview}</pre>
    </div>
    <div class="row gap-8 mt-8" x-show="mode === 'builder'" x-cloak>
      <button type="button" class="btn ghost sm" hx-post="{base_url}/admin/persona/preview" hx-target="#prompt-preview" hx-include="[name='biz_type'],[name='city'],[name='tone'],[name='never'],[name='catch_phrases'],[name='off_topics']">Refresh preview</button>
    </div>
  </div>
</div>"##,
        base_url = base_url,
        safety_badge = safety_badge,
        preset_options = preset_options,
        initial_preview = html_escape(&initial_preview),
        x_data = build_x_data(active_mode, active_preset_slug, &builder, &custom_prompt),
    );

    let page = app_shell(&body, "Persona", base_url);
    base_html("Persona — Concierge", &page)
}

fn render_safety_badge(persona: &PersonaConfig) -> String {
    let prompt_drift_locked = persona.safety.checked_prompt_hash.as_deref()
        != Some(persona.active_prompt_hash().as_str())
        && matches!(persona.safety.status, PersonaSafetyStatus::Approved);

    let (chip_class, label, detail) = if prompt_drift_locked {
        (
            "warn",
            "Re-checking",
            "Prompt was edited; safety re-check in progress.".to_string(),
        )
    } else {
        match &persona.safety.status {
            PersonaSafetyStatus::Approved => (
                "ok",
                "Approved",
                format!(
                    "Last checked {}.",
                    persona.safety.checked_at.as_deref().unwrap_or("just now")
                ),
            ),
            PersonaSafetyStatus::Pending => (
                "warn",
                "Pending",
                "Safety check in progress; AI replies are paused until it completes.".to_string(),
            ),
            PersonaSafetyStatus::Rejected => (
                "warn",
                "Rejected",
                persona
                    .safety
                    .vague_reason
                    .clone()
                    .unwrap_or_else(|| "This persona doesn't fit our content policies.".into()),
            ),
        }
    };

    format!(
        r#"<div class="card p-14 mb-16 row gap-10" style="align-items:center">
  <span class="chip {chip_class}">{label}</span>
  <span class="muted fs-13">{detail}</span>
</div>"#,
        chip_class = chip_class,
        label = html_escape(label),
        detail = html_escape(&detail),
    )
}

fn build_x_data(
    mode: &str,
    preset_slug: &str,
    builder: &crate::types::PersonaBuilder,
    custom_prompt: &str,
) -> String {
    fn esc(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    }
    format!(
        "{{ mode: '{mode}', presetId: '{preset}', customPrompt: '{custom}', builder: {{ biz_type: '{biz}', city: '{city}', tone: '{tone}', never: '{never}', catch_phrases: '{cp}', off_topics: '{ot}' }} }}",
        mode = esc(mode),
        preset = esc(preset_slug),
        custom = esc(custom_prompt),
        biz = esc(&builder.biz_type),
        city = esc(&builder.city),
        tone = esc(&builder.tone),
        never = esc(&builder.never),
        cp = esc(&builder.catch_phrases.join("\n")),
        ot = esc(&builder.off_topics.join("\n")),
    )
}
