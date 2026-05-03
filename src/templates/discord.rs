//! Discord management page: install CTA + channel picker + uninstall.

use crate::helpers::html_escape;
use crate::i18n::t;
use crate::locale::Locale;
use crate::types::DiscordConfig;

use super::base::{app_shell, base_html};
use super::HASH;

/// Render a "Connect Discord" CTA for tenants that have no `DiscordConfig`.
pub fn install_cta_html(from: &str, base_url: &str, locale: &Locale) -> String {
    let install_href = if from.is_empty() {
        format!("{base_url}/admin/discord/install")
    } else {
        format!(
            "{base_url}/admin/discord/install?from={}",
            urlencoding::encode(from)
        )
    };
    let back_href = back_href_for(from, base_url);

    let content = format!(
        r##"<div class="page-pad">
  <p><a href="{back_href}" class="btn ghost sm">{back}</a></p>
  <h1 class="display-sm m-0 mt-8 mb-16">{h1}</h1>
  <div class="card p-22">
    <div class="row gap-16">
      <div class="admin-mark icon-chip">{icon}</div>
      <div class="flex-1">
        <div class="fw-600">{card_h}</div>
        <p class="muted m-0 mt-4 fs-14">{card_lead}</p>
      </div>
      <a href="{install_href}" class="btn primary">{cta}</a>
    </div>
  </div>
</div>"##,
        back_href = back_href,
        install_href = install_href,
        icon = discord_icon(),
        back = t(locale, "admin-discord-install-back"),
        h1 = t(locale, "admin-discord-install-h1"),
        card_h = t(locale, "admin-discord-install-card-headline"),
        card_lead = t(locale, "admin-discord-install-card-lead"),
        cta = t(locale, "admin-discord-install-cta"),
    );

    let page = app_shell(&content, "Settings", base_url, locale);
    base_html(&t(locale, "admin-discord-install-title"), &page, locale)
}

/// Render the installed-management page: show guild, pick channels, uninstall.
pub fn manage_html(
    cfg: &DiscordConfig,
    channels: &[botrelay::discord::GuildChannel],
    from: &str,
    base_url: &str,
    locale: &Locale,
) -> String {
    let back_href = back_href_for(from, base_url);
    let guild = cfg.guild_name.as_deref().unwrap_or("Discord server");

    let options_html = |selected: Option<&str>| -> String {
        let mut out = String::from(r#"<option value="">(none)</option>"#);
        for ch in channels {
            let sel = if Some(ch.id.as_str()) == selected {
                " selected"
            } else {
                ""
            };
            out.push_str(&format!(
                r#"<option value="{id}"{sel}>#{name}</option>"#,
                id = html_escape(&ch.id),
                sel = sel,
                name = html_escape(&ch.name),
            ));
        }
        out
    };

    let approval_opts = options_html(cfg.approval_channel_id.as_deref());

    let inbound_channel_opts: String = channels
        .iter()
        .map(|ch| {
            let sel = if cfg.inbound_channel_ids.contains(&ch.id) {
                " selected"
            } else {
                ""
            };
            format!(
                r#"<option value="{id}"{sel}>#{name}</option>"#,
                id = html_escape(&ch.id),
                sel = sel,
                name = html_escape(&ch.name),
            )
        })
        .collect();

    let empty_note = if channels.is_empty() {
        format!(
            r#"<p class="muted mt-8 fs-13">{}</p>"#,
            t(locale, "admin-discord-empty-channels"),
        )
    } else {
        String::new()
    };

    let content = format!(
        r##"<div class="page-pad">
  <p><a href="{back_href}" class="btn ghost sm">{back}</a></p>
  <div class="between m-0 mt-8 mb-16">
    <div>
      <h1 class="display-sm m-0">{h1}</h1>
      <p class="muted m-0 mt-4 fs-13">{server_prefix} <strong>{guild}</strong></p>
    </div>
    <div class="row gap-8">
      <a href="{base_url}/admin/discord/install?from={from_enc}" class="btn ghost sm">{reinstall}</a>
      <button class="btn ghost sm text-warn" hx-delete="{base_url}/admin/discord" hx-confirm="{uninstall_confirm}">{uninstall}</button>
    </div>
  </div>

  <form class="card p-22" hx-put="{base_url}/admin/discord/config" hx-ext="json-enc" hx-target="{hash}dc-toast" hx-swap="innerHTML"
        x-data="{{ ar_enabled: {ar_enabled}, ar_mode: '{ar_mode}', wait_seconds: {wait}, mentions: {inbound_mentions}, channels: {inbound_channels_json} }}">
    <h3 class="m-0 mb-12">{out_h3}</h3>
    <p class="muted m-0 mb-16 fs-14">{out_lead}</p>

    <div class="form-group">
      <label for="dc-approval-channel" class="eyebrow lbl">{approval_lbl}</label>
      <select id="dc-approval-channel" class="select" name="approval_channel_id">{approval_opts}</select>
      <small class="muted fs-12">{approval_help}</small>
    </div>

    <h3 class="m-0 mt-24 mb-12">{in_h3}</h3>
    <p class="muted m-0 mb-16 fs-14">{in_lead}</p>

    <div class="form-group">
      <label><input type="checkbox" name="inbound_mentions" value="true" x-model="mentions"> {in_mentions}</label>
      <input type="hidden" name="inbound_mentions_present" value="true">
    </div>

    <div class="form-group">
      <label for="dc-inbound-channels" class="eyebrow lbl">{in_channels_lbl}</label>
      <select id="dc-inbound-channels" class="select" name="inbound_channel_ids" multiple size="6" x-model="channels">{channel_opts}</select>
      <small class="muted fs-12">{in_channels_help}</small>
    </div>

    <h3 class="m-0 mt-24 mb-12">{ar_h3}</h3>
    <p class="muted fs-13 mb-12">{ar_rules_prefix} <a href="{base_url}/admin/rules/discord/_">{ar_rules_link}</a>.</p>

    <div class="form-group">
      <label><input type="checkbox" name="auto_reply_enabled" value="true" x-model="ar_enabled"> {ar_enabled_lbl}</label>
    </div>

    <div class="form-group" x-show="ar_enabled" x-cloak :aria-hidden="!ar_enabled">
      <label for="dc-ar-mode" class="eyebrow lbl">{ar_mode_lbl}</label>
      <select id="dc-ar-mode" class="select" name="auto_reply_mode" x-model="ar_mode">
        <option value="canned">{ar_mode_canned}</option>
        <option value="prompt">{ar_mode_prompt}</option>
      </select>
    </div>

    <div class="form-group" x-show="ar_enabled" x-cloak :aria-hidden="!ar_enabled">
      <label for="dc-ar-prompt" class="eyebrow lbl"><span x-text="ar_mode === 'prompt' ? '{ar_prompt_system}' : '{ar_prompt_reply}'"></span></label>
      <textarea id="dc-ar-prompt" class="textarea" name="auto_reply_prompt" rows="3">{ar_prompt}</textarea>
    </div>

    <div class="form-group" x-show="ar_enabled" x-cloak :aria-hidden="!ar_enabled">
      <label for="dc-wait" class="eyebrow lbl">{wait_prefix} <span x-text="wait_seconds === 0 ? '{wait_instant}' : wait_seconds + 's'"></span></label>
      <input id="dc-wait" type="range" min="0" max="30" step="1" name="wait_seconds" x-model.number="wait_seconds" style="accent-color:var(--accent)">
      <small class="muted fs-12">{wait_help}</small>
    </div>

    {empty_note}

    <div class="row gap-8 mt-16" style="justify-content:flex-end">
      <button type="submit" class="btn primary">{save}</button>
    </div>
    <div id="dc-toast" class="mt-8" role="status" aria-live="polite" aria-atomic="true"></div>
  </form>
</div>"##,
        back_href = back_href,
        base_url = base_url,
        guild = html_escape(guild),
        from_enc = urlencoding::encode(from),
        approval_opts = approval_opts,
        channel_opts = inbound_channel_opts,
        empty_note = empty_note,
        hash = HASH,
        ar_enabled = if cfg.auto_reply.enabled {
            "true"
        } else {
            "false"
        },
        ar_mode = if cfg.auto_reply.default_is_canned() {
            "canned"
        } else {
            "prompt"
        },
        ar_prompt = html_escape(cfg.auto_reply.default_text()),
        wait = cfg.auto_reply.wait_seconds,
        inbound_mentions = if cfg.inbound_mentions {
            "true"
        } else {
            "false"
        },
        inbound_channels_json =
            serde_json::to_string(&cfg.inbound_channel_ids).unwrap_or_else(|_| "[]".into()),
        back = t(locale, "admin-discord-manage-back"),
        h1 = t(locale, "admin-discord-install-h1"),
        server_prefix = t(locale, "admin-discord-manage-server-prefix"),
        reinstall = t(locale, "admin-discord-install-cta"),
        uninstall = t(locale, "admin-discord-manage-uninstall"),
        uninstall_confirm = html_escape(&t(locale, "admin-discord-manage-uninstall-confirm")),
        out_h3 = t(locale, "admin-discord-out-h3"),
        out_lead = t(locale, "admin-discord-out-lead"),
        approval_lbl = t(locale, "admin-discord-approval-channel"),
        approval_help = t(locale, "admin-discord-approval-channel-help"),
        in_h3 = t(locale, "admin-discord-in-h3"),
        in_lead = t(locale, "admin-discord-in-lead"),
        in_mentions = t(locale, "admin-discord-in-mentions"),
        in_channels_lbl = t(locale, "admin-discord-in-channels"),
        in_channels_help = t(locale, "admin-discord-in-channels-help"),
        ar_h3 = t(locale, "admin-discord-ar-h3"),
        ar_rules_prefix = t(locale, "admin-discord-ar-rules-prefix"),
        ar_rules_link = t(locale, "admin-discord-ar-rules-link"),
        ar_enabled_lbl = t(locale, "admin-discord-ar-enabled"),
        ar_mode_lbl = t(locale, "admin-discord-ar-mode"),
        ar_mode_canned = t(locale, "admin-discord-ar-mode-canned"),
        ar_mode_prompt = t(locale, "admin-discord-ar-mode-prompt"),
        ar_prompt_system = t(locale, "admin-discord-ar-prompt-system"),
        ar_prompt_reply = t(locale, "admin-discord-ar-prompt-reply"),
        wait_prefix = t(locale, "admin-discord-ar-wait-prefix"),
        wait_instant = t(locale, "admin-discord-ar-wait-instant"),
        wait_help = t(locale, "admin-discord-ar-wait-help"),
        save = t(locale, "admin-discord-save"),
    );

    let page = app_shell(&content, "Settings", base_url, locale);
    base_html(&t(locale, "admin-discord-manage-title"), &page, locale)
}

fn back_href_for(from: &str, base_url: &str) -> String {
    match from {
        "wizard_channels" => format!("{base_url}/admin/wizard/channels"),
        "wizard_heads_up" => format!("{base_url}/admin/wizard/notifications"),
        _ => format!("{base_url}/admin/settings"),
    }
}

/// Discord's squircle SVG, reused from onboarding.rs's channel_icon.
fn discord_icon() -> &'static str {
    r#"<svg width="22" height="22" viewBox="0 0 24 24" fill="none"><path d="M7 7c1.4-.7 3-1 5-1s3.6.3 5 1l1 1 1.5 4.5c.2 2-.3 3.8-1.5 5.5-1 .3-2 .5-3 .5l-1-1.5c.5-.2 1-.4 1.5-.8-.3-.2-.8-.4-1.2-.5-2 .7-4.6.7-6.6 0-.4.1-.9.3-1.2.5.5.4 1 .6 1.5.8L6 17.5c-1 0-2-.2-3-.5-1.2-1.7-1.7-3.5-1.5-5.5L3 7l1-1z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/></svg>"#
}
