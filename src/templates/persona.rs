//! `/admin/persona` — render the three-mode (Preset/Builder/Custom)
//! persona editor with a live prompt preview and a safety badge.
//!
//! The page is a single Alpine `x-data` form; the active mode swaps the
//! visible fields. Submission posts to the persona admin handler which
//! decides which source variant to construct.

use crate::helpers::html_escape;
use crate::i18n::t;
use crate::locale::Locale;
use crate::personas;
use crate::types::{PersonaConfig, PersonaPreset, PersonaSafetyStatus, PersonaSource};

use super::base::{app_shell, base_html};

pub fn persona_admin_html(persona: &PersonaConfig, base_url: &str, locale: &Locale) -> String {
    // After the archetype refactor, `PersonaSource::Preset` is gone — picking
    // an archetype card just stamps it onto a `Builder` source. We pre-fill
    // the form depending on which source variant the tenant has saved.
    let (active_mode, builder, custom_prompt) = match &persona.source {
        PersonaSource::Builder(b) => ("builder", b.clone(), String::new()),
        PersonaSource::Custom(s) => ("custom", crate::types::PersonaBuilder::default(), s.clone()),
    };
    let active_archetype_slug = builder.archetype.slug();

    let archetype_options: String = PersonaPreset::ALL
        .iter()
        .map(|p| {
            let slug = p.slug();
            let label = p.label();
            let desc = p.description();
            format!(
                r#"<label class="card p-14" style="cursor:pointer;display:block">
  <div class="row gap-10" style="align-items:flex-start">
    <input type="radio" name="archetype" value="{slug}" x-model="builder.archetype" style="margin-top:4px">
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
        PersonaSource::Builder(b) => personas::generate(b),
        PersonaSource::Custom(s) => s.clone(),
    };
    let _ = active_archetype_slug;

    let safety_badge = render_safety_badge(persona, locale);

    let body = format!(
        r##"<div class="page-pad" x-data="{x_data}" hx-ext="json-enc">
  <p><a href="{base_url}/admin" class="btn ghost sm">{back}</a></p>
  <h1 class="display-sm m-0 mb-4">{h1}</h1>
  <p class="muted mb-16">{lead}</p>

  {safety_badge}

  <div class="card p-22 mb-16">
    <div class="eyebrow mb-12" id="persona-mode-label">{mode_eyebrow}</div>
    <div class="row gap-8 mb-16" style="flex-wrap:wrap" role="radiogroup" aria-labelledby="persona-mode-label">
      <label class="row gap-6"><input type="radio" name="mode" value="builder" x-model="mode"> {mode_builder}</label>
      <label class="row gap-6"><input type="radio" name="mode" value="custom" x-model="mode"> {mode_custom}</label>
    </div>

    <form hx-post="{base_url}/admin/persona" hx-target="body" hx-swap="innerHTML">
      <input type="hidden" name="mode" :value="mode">

      <!-- BUILDER -->
      <div x-show="mode === 'builder'" x-cloak :aria-hidden="mode !== 'builder'">
        <p class="muted fs-13 mb-12">{builder_lead}</p>
        <div class="eyebrow lbl mb-6">{lbl_archetype}</div>
        <div style="display:grid;gap:10px;grid-template-columns:1fr 1fr">
          {archetype_options}
        </div>
        <div class="mt-12" style="display:grid;grid-template-columns:1fr 1fr;gap:12px">
          <div>
            <label for="persona-biz-name" class="eyebrow lbl">{lbl_biz_name}</label>
            <input id="persona-biz-name" class="input" name="biz_name" x-model="builder.biz_name" placeholder="{ph_biz_name}" required>
          </div>
          <div>
            <label for="persona-biz-type" class="eyebrow lbl">{lbl_biz_type}</label>
            <input id="persona-biz-type" class="input" name="biz_type" x-model="builder.biz_type" placeholder="{ph_biz_type}" required>
          </div>
          <div>
            <label for="persona-city" class="eyebrow lbl">{lbl_city}</label>
            <input id="persona-city" class="input" name="city" x-model="builder.city" placeholder="{ph_city}">
          </div>
          <div>
            <label for="persona-never" class="eyebrow lbl">{lbl_never}</label>
            <input id="persona-never" class="input" name="never" x-model="builder.never" placeholder="{ph_never}">
          </div>
        </div>
        <div class="mt-12">
          <label for="persona-catch-phrases" class="eyebrow lbl">{lbl_catch}</label>
          <textarea id="persona-catch-phrases" class="textarea" name="catch_phrases" x-model="builder.catch_phrases" rows="3" placeholder="{ph_catch}"></textarea>
        </div>
        <div class="mt-12">
          <label for="persona-off-topics" class="eyebrow lbl">{lbl_off}</label>
          <textarea id="persona-off-topics" class="textarea" name="off_topics" x-model="builder.off_topics" rows="3" placeholder="{ph_off}"></textarea>
        </div>
      </div>

      <!-- CUSTOM -->
      <div x-show="mode === 'custom'" x-cloak :aria-hidden="mode !== 'custom'">
        <p class="muted fs-13 mb-12">{custom_lead}</p>
        <label for="persona-custom-prompt" class="sr-only">{custom_sr}</label>
        <textarea id="persona-custom-prompt" class="textarea mono" name="custom_prompt" x-model="customPrompt" rows="12" maxlength="2000" placeholder="{custom_ph}"></textarea>
        <p class="muted fs-12 mt-4"><span x-text="customPrompt.length"></span> / 2000</p>
      </div>

      <div class="row gap-8 mt-16" style="justify-content:flex-end">
        <button type="submit" class="btn primary">{save}</button>
      </div>
    </form>
  </div>

  <div class="card p-14" style="background:var(--ink);color:var(--cream);border-color:var(--ink);border-radius:var(--r-sm)">
    <div class="mono fs-10 mb-6" style="letter-spacing:.18em;color:var(--accent-soft)">{preview_eyebrow}</div>
    <p class="fs-12 mb-8" style="color:var(--cream);opacity:.7;margin-top:0">{envelope_note}</p>
    <pre class="prompt-preview prompt-preview-fixed" aria-label="{preamble_label}">{preamble}</pre>
    <pre id="prompt-preview" class="prompt-preview prompt-preview-middle">{initial_preview}</pre>
    <pre class="prompt-preview prompt-preview-fixed" aria-label="{postamble_label}">{postamble}</pre>
    <div class="row gap-8 mt-8" x-show="mode === 'builder'" x-cloak :aria-hidden="mode !== 'builder'">
      <button type="button" class="btn ghost sm" hx-post="{base_url}/admin/persona/preview" hx-target="#prompt-preview" hx-swap="outerHTML" hx-include="[name='archetype'],[name='biz_name'],[name='biz_type'],[name='city'],[name='never'],[name='catch_phrases'],[name='off_topics']">{refresh}</button>
    </div>
  </div>
</div>"##,
        base_url = base_url,
        safety_badge = safety_badge,
        archetype_options = archetype_options,
        initial_preview = html_escape(&initial_preview),
        x_data = build_x_data(active_mode, &builder, &custom_prompt),
        back = t(locale, "admin-persona-back"),
        h1 = t(locale, "admin-persona-h1"),
        lead = t(locale, "admin-persona-lead"),
        mode_eyebrow = t(locale, "admin-persona-mode-eyebrow"),
        mode_builder = t(locale, "admin-persona-mode-builder"),
        mode_custom = t(locale, "admin-persona-mode-custom"),
        builder_lead = t(locale, "admin-persona-builder-lead"),
        lbl_archetype = t(locale, "admin-persona-label-archetype"),
        lbl_biz_name = t(locale, "admin-persona-label-biz-name"),
        lbl_biz_type = t(locale, "admin-persona-label-biz-type"),
        lbl_city = t(locale, "admin-persona-label-city"),
        lbl_never = t(locale, "admin-persona-label-never"),
        lbl_catch = t(locale, "admin-persona-label-catch-phrases"),
        lbl_off = t(locale, "admin-persona-label-off-topics"),
        ph_biz_name = t(locale, "admin-persona-placeholder-biz-name"),
        ph_biz_type = t(locale, "admin-persona-placeholder-biz-type"),
        ph_city = t(locale, "admin-persona-placeholder-city"),
        ph_never = t(locale, "admin-persona-placeholder-never"),
        ph_catch = t(locale, "admin-persona-placeholder-catch-phrases"),
        ph_off = t(locale, "admin-persona-placeholder-off-topics"),
        custom_lead = t(locale, "admin-persona-custom-lead"),
        custom_sr = t(locale, "admin-persona-custom-sr-only"),
        custom_ph = t(locale, "admin-persona-custom-placeholder"),
        save = t(locale, "admin-persona-save"),
        preview_eyebrow = t(locale, "admin-persona-preview-eyebrow"),
        envelope_note = html_escape(&t(locale, "admin-persona-envelope-note")),
        preamble_label = html_escape(&t(locale, "admin-prompt-preamble-label")),
        postamble_label = html_escape(&t(locale, "admin-prompt-postamble-label")),
        preamble = html_escape(crate::prompt::PREAMBLE),
        postamble = html_escape(crate::prompt::POSTAMBLE),
        refresh = t(locale, "admin-persona-preview-refresh"),
    );

    let page = app_shell(&body, "Persona", base_url, locale);
    base_html(&t(locale, "admin-persona-title"), &page, locale)
}

fn render_safety_badge(persona: &PersonaConfig, locale: &Locale) -> String {
    let prompt_drift_locked = persona.safety.checked_prompt_hash.as_deref()
        != Some(persona.active_prompt_hash().as_str())
        && matches!(persona.safety.status, PersonaSafetyStatus::Approved);

    let (chip_class, label, detail) = if prompt_drift_locked {
        (
            "warn",
            t(locale, "admin-persona-safety-rechecking"),
            t(locale, "admin-persona-safety-rechecking-detail"),
        )
    } else {
        match &persona.safety.status {
            PersonaSafetyStatus::Approved => (
                "ok",
                t(locale, "admin-persona-safety-approved"),
                format!(
                    "{} {}.",
                    t(locale, "admin-persona-safety-approved-prefix"),
                    persona
                        .safety
                        .checked_at
                        .clone()
                        .unwrap_or_else(|| t(locale, "admin-persona-safety-approved-fallback"))
                ),
            ),
            PersonaSafetyStatus::Pending => (
                "warn",
                t(locale, "admin-persona-safety-pending"),
                t(locale, "admin-persona-safety-pending-detail"),
            ),
            PersonaSafetyStatus::Rejected => (
                "warn",
                t(locale, "admin-persona-safety-rejected"),
                persona
                    .safety
                    .vague_reason
                    .clone()
                    .unwrap_or_else(|| t(locale, "admin-persona-safety-rejected-fallback")),
            ),
        }
    };

    format!(
        r#"<div class="card p-14 mb-16 row gap-10" style="align-items:center">
  <span class="chip {chip_class}">{label}</span>
  <span class="muted fs-13">{detail}</span>
</div>"#,
        chip_class = chip_class,
        label = html_escape(&label),
        detail = html_escape(&detail),
    )
}

fn build_x_data(mode: &str, builder: &crate::types::PersonaBuilder, custom_prompt: &str) -> String {
    fn esc(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    }
    format!(
        "{{ mode: '{mode}', customPrompt: '{custom}', builder: {{ archetype: '{archetype}', biz_name: '{biz_name}', biz_type: '{biz_type}', city: '{city}', never: '{never}', catch_phrases: '{cp}', off_topics: '{ot}' }} }}",
        mode = esc(mode),
        custom = esc(custom_prompt),
        archetype = esc(builder.archetype.slug()),
        biz_name = esc(&builder.biz_name),
        biz_type = esc(&builder.biz_type),
        city = esc(&builder.city),
        never = esc(&builder.never),
        cp = esc(&builder.catch_phrases.join("\n")),
        ot = esc(&builder.off_topics.join("\n")),
    )
}
