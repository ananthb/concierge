//! Admin dashboard templates for messaging-first product

use crate::helpers::html_escape;
use crate::i18n::t;
use crate::locale::Locale;
use crate::types::*;

use super::base::base_html;
use super::HASH;

pub fn auth_login_html(
    base_url: &str,
    google_client_id: &str,
    meta_app_id: &str,
    wa_config_id: &str,
    wa_state: &str,
    last_provider: Option<&str>,
    locale: &Locale,
) -> String {
    let redirect_uri = format!("{}/auth/callback", base_url);
    let google_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=online&prompt=select_account",
        urlencoding::encode(google_client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("openid email profile"),
    );
    let fb_redirect_uri = format!("{}/auth/facebook/callback", base_url);
    let fb_url = format!(
        "https://www.facebook.com/{}/dialog/oauth?client_id={}&redirect_uri={}&scope=email&response_type=code",
        crate::META_API_VERSION,
        urlencoding::encode(meta_app_id),
        urlencoding::encode(&fb_redirect_uri),
    );

    // Brand-correct icons. Each renders well on its own button background:
    // Google = white button + multicolor G; Facebook = blue button + white f;
    // WhatsApp = green button + white speech-bubble.
    let google_svg = r##"<svg width="18" height="18" viewBox="0 0 18 18" xmlns="http://www.w3.org/2000/svg" aria-hidden="true"><path d="M17.64 9.2c0-.637-.057-1.251-.164-1.84H9v3.481h4.844a4.14 4.14 0 01-1.796 2.716v2.259h2.908c1.702-1.567 2.684-3.875 2.684-6.615z" fill="#4285F4"/><path d="M9 18c2.43 0 4.467-.806 5.956-2.18l-2.908-2.259c-.806.54-1.837.86-3.048.86-2.344 0-4.328-1.584-5.036-3.711H.957v2.332A8.997 8.997 0 009 18z" fill="#34A853"/><path d="M3.964 10.71A5.41 5.41 0 013.682 9c0-.593.102-1.17.282-1.71V4.958H.957A8.996 8.996 0 000 9c0 1.452.348 2.827.957 4.042l3.007-2.332z" fill="#FBBC05"/><path d="M9 3.58c1.321 0 2.508.454 3.44 1.345l2.582-2.58C13.463.891 11.426 0 9 0A8.997 8.997 0 00.957 4.958L3.964 7.29C4.672 5.163 6.656 3.58 9 3.58z" fill="#EA4335"/></svg>"##;
    let fb_svg = r##"<svg width="18" height="18" viewBox="0 0 24 24" fill="#fff" xmlns="http://www.w3.org/2000/svg" aria-hidden="true"><path d="M13.397 20.997v-8.196h2.765l.411-3.209h-3.176V7.548c0-.926.258-1.56 1.587-1.56h1.684V3.127A22.336 22.336 0 0 0 14.201 3c-2.444 0-4.122 1.492-4.122 4.231v2.355H7.332v3.209h2.753v8.202h3.312z"/></svg>"##;
    let wa_svg = r##"<svg width="18" height="18" viewBox="0 0 24 24" fill="#fff" xmlns="http://www.w3.org/2000/svg" aria-hidden="true"><path d="M17.472 14.382c-.297-.149-1.758-.867-2.03-.967-.273-.099-.471-.148-.67.15-.197.297-.767.966-.94 1.164-.173.199-.347.223-.644.075-.297-.15-1.255-.463-2.39-1.475-.883-.788-1.48-1.761-1.653-2.059-.173-.297-.018-.458.13-.606.134-.133.298-.347.446-.52.149-.174.198-.298.298-.497.099-.198.05-.371-.025-.52-.075-.149-.669-1.612-.916-2.207-.242-.579-.487-.5-.669-.51-.173-.008-.371-.01-.57-.01-.198 0-.52.074-.792.372-.272.297-1.04 1.016-1.04 2.479 0 1.462 1.065 2.875 1.213 3.074.149.198 2.096 3.2 5.077 4.487.709.306 1.262.489 1.694.625.712.227 1.36.195 1.871.118.571-.085 1.758-.719 2.006-1.413.248-.694.248-1.289.173-1.413-.074-.124-.272-.198-.57-.347m-5.421 7.403h-.004a9.87 9.87 0 0 1-5.031-1.378l-.361-.214-3.741.982.998-3.648-.235-.374a9.86 9.86 0 0 1-1.51-5.26c.001-5.45 4.436-9.884 9.888-9.884 2.64 0 5.122 1.03 6.988 2.898a9.825 9.825 0 0 1 2.893 6.994c-.003 5.45-4.437 9.884-9.885 9.884m8.413-18.297A11.815 11.815 0 0 0 12.05 0C5.495 0 .16 5.335.157 11.892c0 2.096.547 4.142 1.588 5.945L.057 24l6.305-1.654a11.882 11.882 0 0 0 5.683 1.448h.005c6.554 0 11.89-5.335 11.893-11.893a11.821 11.821 0 0 0-3.48-8.413z"/></svg>"##;

    let google_label = t(locale, "admin-login-google");
    let fb_label = t(locale, "admin-login-facebook-continue");
    let wa_label = t(locale, "admin-login-whatsapp");

    let google_btn = format!(
        r#"<a href="{google_url}" class="btn brand-google lg w-full jc-center">{google_svg} {google_label}</a>"#,
        google_url = html_escape(&google_url),
    );
    let fb_btn = format!(
        r#"<a href="{fb_url}" class="btn brand-facebook lg w-full jc-center">{fb_svg} {fb_label}</a>"#,
        fb_url = html_escape(&fb_url),
    );
    // WhatsApp button is JS-driven (Meta Embedded Signup). If the page lacks
    // the Meta app id, omit the button rather than render a dud. The click
    // handler is wired via addEventListener in the module script below — no
    // inline onclick — so the module can keep its handler closures private.
    let wa_btn = if meta_app_id.is_empty() {
        String::new()
    } else {
        format!(
            r#"<button type="button" id="wa-signup-btn" class="btn brand-whatsapp lg w-full jc-center">{wa_svg} {wa_label}</button>"#,
        )
    };

    // Order buttons so the user's last-used provider is on top, but every
    // button shares the same brand-coloured weight so none looks "preferred".
    let buttons = match last_provider {
        Some("facebook") => format!("{fb_btn}\n      {google_btn}\n      {wa_btn}"),
        Some("whatsapp") => format!("{wa_btn}\n      {google_btn}\n      {fb_btn}"),
        _ => format!("{google_btn}\n      {fb_btn}\n      {wa_btn}"),
    };

    let wa_error = t(locale, "admin-login-whatsapp-error");
    let wa_connecting = t(locale, "admin-login-whatsapp-connecting");
    let wa_script = if meta_app_id.is_empty() {
        String::new()
    } else {
        format!(
            r#"<script async defer crossorigin="anonymous" src="https://connect.facebook.net/en_US/sdk.js"></script>
<script type="module" nonce="__CSP_NONCE__">
// The SDK loads `async defer` — an early click would otherwise hit
// `FB is not defined`. We expose a Promise resolved from inside
// `fbAsyncInit` and have the click handler await it (with a timeout
// so a blocked / failed load surfaces an error instead of hanging).
//
// Module scripts are deferred, so the SDK can run *before* this code
// executes. Handle both orderings: if FB is already loaded, init now;
// otherwise let the SDK call our hook when it loads.
const fbReady = new Promise((resolve) => {{
  const init = () => {{
    FB.init({{ appId: '{app_id}', autoLogAppEvents: true, xfbml: true, version: '{api_version}' }});
    resolve();
  }};
  if (typeof FB !== 'undefined' && typeof FB.init === 'function') init();
  else window.fbAsyncInit = init;
}});

const btn = document.getElementById('wa-signup-btn');
const errDiv = document.getElementById('wa-signup-error');
const SDK_TIMEOUT_MS = 8000;

const buildLoginConfig = () => {{
  const cfg = {{
    response_type: 'code',
    override_default_response_type: true,
    extras: {{ featureType: '', sessionInfoVersion: '3' }},
  }};
  const configId = '{config_id}';
  if (configId) cfg.config_id = configId;
  else cfg.scope = 'whatsapp_business_management,whatsapp_business_messaging,email,public_profile';
  return cfg;
}};

const submitCallback = ({{ code, phone_number_id }}) => {{
  const form = document.createElement('form');
  form.method = 'POST';
  form.action = '{base_url}/whatsapp/signup/public-callback';
  const append = (name, value) => {{
    const input = document.createElement('input');
    input.type = 'hidden';
    input.name = name;
    input.value = value;
    form.appendChild(input);
  }};
  append('code', code);
  append('state', '{state}');
  if (phone_number_id) append('phone_number_id', phone_number_id);
  document.body.appendChild(form);
  form.submit();
}};

btn?.addEventListener('click', async () => {{
  const origLabel = btn.innerHTML;
  btn.disabled = true;
  btn.textContent = '{connecting}';
  errDiv.textContent = '';

  const fail = () => {{
    btn.disabled = false;
    btn.innerHTML = origLabel;
    errDiv.textContent = '{error}';
  }};

  try {{
    const timeout = new Promise((_, reject) =>
      setTimeout(() => reject(new Error('sdk-timeout')), SDK_TIMEOUT_MS));
    await Promise.race([fbReady, timeout]);
  }} catch {{
    fail();
    return;
  }}

  FB.login((response) => {{
    if (response.authResponse?.code) submitCallback(response.authResponse);
    else fail();
  }}, buildLoginConfig());
}});
</script>"#,
            app_id = html_escape(meta_app_id),
            api_version = crate::META_API_VERSION,
            config_id = html_escape(wa_config_id),
            base_url = html_escape(base_url),
            state = html_escape(wa_state),
            connecting = html_escape(&wa_connecting),
            error = html_escape(&wa_error),
        )
    };

    let content = format!(
        r#"<div class="ta-center" style="max-width:360px;margin:4rem auto;padding:0 1rem">
    <div style="margin-bottom:2rem">{logo}
    <div class="serif mt-8" style="font-size:28px">Concierge</div></div>
    <p class="muted" style="margin-bottom:2rem">{tagline}</p>
    <div id="wa-signup-error" class="text-warn mb-12" role="alert" aria-live="assertive"></div>
    <div class="stack gap-12">
      {buttons}
    </div>
    <a href="/" class="btn ghost sm mt-24">{back}</a>
</div>
{wa_script}"#,
        logo = super::base::LOGO_INLINE,
        buttons = buttons,
        tagline = t(locale, "admin-login-tagline"),
        back = t(locale, "admin-login-back"),
    );

    base_html(&t(locale, "admin-login-title"), &content, locale)
}

/// Render the "Conversation Timing" card on /admin/settings.
///
/// Three optional integer inputs map directly to `ConversationConfig`'s
/// `Option<u32>` fields. Empty value = `None` = use the prompt default,
/// shown in placeholder + an explicit "Default: N" hint so the tenant
/// can see what they'll fall back to without saving. Submits as JSON
/// (HTMX json-enc) PUT to `/admin/settings/conversation`; the
/// response is an inline fragment (success/error toast) swapped into
/// the per-card toast div.
fn conversation_settings_card(base_url: &str, cfg: &ConversationConfig, locale: &Locale) -> String {
    let h2 = t(locale, "admin-settings-conversation-h2");
    let lead = t(locale, "admin-settings-conversation-lead");
    let idle_label = t(locale, "admin-settings-conversation-idle-label");
    let idle_help = t(locale, "admin-settings-conversation-idle-help");
    let cooldown_label = t(locale, "admin-settings-conversation-cooldown-label");
    let cooldown_help = t(locale, "admin-settings-conversation-cooldown-help");
    let history_label = t(locale, "admin-settings-conversation-history-label");
    let history_help = t(locale, "admin-settings-conversation-history-help");
    let unit_mins = t(locale, "admin-settings-conversation-unit-mins");
    let unit_turns = t(locale, "admin-settings-conversation-unit-turns");
    let default_prefix = t(locale, "admin-settings-conversation-default-prefix");
    let save = t(locale, "admin-save");

    // Defaults shown both as input placeholder and inline hint —
    // tenants who haven't set a value see what number actually gets
    // used at runtime.
    let default_idle = crate::prompt::DEFAULT_CONVERSATION_IDLE_GAP_MINS;
    let default_cooldown = crate::prompt::DEFAULT_HANDOFF_COOLDOWN_MINS;
    let default_history = crate::prompt::DEFAULT_CONVERSATION_MAX_MESSAGES;

    let idle_value = cfg.idle_gap_mins.map(|v| v.to_string()).unwrap_or_default();
    let cooldown_value = cfg
        .handoff_cooldown_mins
        .map(|v| v.to_string())
        .unwrap_or_default();
    let history_value = cfg
        .max_history_messages
        .map(|v| v.to_string())
        .unwrap_or_default();

    format!(
        r#"<div class="card p-22" hx-ext="json-enc">
            <h2>{h2}</h2>
            <p class="muted mb-16">{lead}</p>
            <form hx-put="{base_url}/admin/settings/conversation" hx-target="{hash}conversation-toast" hx-swap="innerHTML">
                <div class="stack gap-16">
                    <label class="stack gap-4">
                        <span><strong>{idle_label}</strong></span>
                        <div class="row gap-8" style="align-items:center">
                            <input class="input" type="number" name="idle_gap_mins"
                                   inputmode="numeric" min="5" max="1440"
                                   value="{idle_value}" placeholder="{default_idle}"
                                   style="width:8rem">
                            <span class="muted">{unit_mins}</span>
                            <span class="muted fs-13">{default_prefix} {default_idle}</span>
                        </div>
                        <span class="muted fs-13">{idle_help}</span>
                    </label>
                    <label class="stack gap-4">
                        <span><strong>{cooldown_label}</strong></span>
                        <div class="row gap-8" style="align-items:center">
                            <input class="input" type="number" name="handoff_cooldown_mins"
                                   inputmode="numeric" min="5" max="1440"
                                   value="{cooldown_value}" placeholder="{default_cooldown}"
                                   style="width:8rem">
                            <span class="muted">{unit_mins}</span>
                            <span class="muted fs-13">{default_prefix} {default_cooldown}</span>
                        </div>
                        <span class="muted fs-13">{cooldown_help}</span>
                    </label>
                    <label class="stack gap-4">
                        <span><strong>{history_label}</strong></span>
                        <div class="row gap-8" style="align-items:center">
                            <input class="input" type="number" name="max_history_messages"
                                   inputmode="numeric" min="1" max="200"
                                   value="{history_value}" placeholder="{default_history}"
                                   style="width:8rem">
                            <span class="muted">{unit_turns}</span>
                            <span class="muted fs-13">{default_prefix} {default_history}</span>
                        </div>
                        <span class="muted fs-13">{history_help}</span>
                    </label>
                    <div>
                        <button type="submit" class="btn sm">{save}</button>
                    </div>
                </div>
            </form>
            <div id="conversation-toast" class="mt-8" role="status" aria-live="polite" aria-atomic="true"></div>
        </div>"#,
        hash = HASH,
    )
}

pub fn admin_settings_html(
    tenant: &Tenant,
    base_url: &str,
    google_client_id: &str,
    meta_app_id: &str,
    wa: &[WhatsAppAccount],
    ig: &[InstagramAccount],
    discord: Option<&DiscordConfig>,
    conversation: &ConversationConfig,
    tenant_id: &str,
    locale: &Locale,
) -> String {
    let has_google = !tenant.email.is_empty();
    let has_facebook = tenant.facebook_id.is_some();

    // Integrations section reuses the wizard's channel card helper, so
    // Connect/Manage behaves identically to the onboarding Channels step.
    use super::onboarding::{channel_card_html, ChannelCardProps};
    let ig_name = t(locale, "wizard-channels-name-instagram");
    let ig_flavor = t(locale, "wizard-channels-flavor-instagram");
    let wa_name = t(locale, "wizard-channels-name-whatsapp");
    let wa_flavor = t(locale, "wizard-channels-flavor-whatsapp");
    let dc_name = t(locale, "wizard-channels-name-discord");
    let dc_flavor = t(locale, "wizard-channels-flavor-discord");
    let dc_fallback = t(locale, "wizard-channels-discord-handle-fallback");
    let ig_card = {
        let ig_handle = ig
            .first()
            .map(|a| format!("@{}", a.instagram_username))
            .unwrap_or_default();
        let connect = format!(
            "{base_url}/instagram/auth/{}",
            crate::helpers::html_escape(tenant_id)
        );
        let manage = format!("{base_url}/admin/instagram");
        channel_card_html(
            &ChannelCardProps {
                key: "ig",
                name: &ig_name,
                connected: !ig.is_empty(),
                status_line: if ig.is_empty() {
                    &ig_flavor
                } else {
                    &ig_handle
                },
                connect_href: &connect,
                manage_href: &manage,
            },
            locale,
        )
    };
    let wa_card = {
        let wa_handle = wa
            .first()
            .map(|a| a.phone_number.clone())
            .unwrap_or_default();
        let connect = format!("{base_url}/admin/whatsapp/new");
        let manage = format!("{base_url}/admin/whatsapp");
        channel_card_html(
            &ChannelCardProps {
                key: "wa",
                name: &wa_name,
                connected: !wa.is_empty(),
                status_line: if wa.is_empty() {
                    &wa_flavor
                } else {
                    &wa_handle
                },
                connect_href: &connect,
                manage_href: &manage,
            },
            locale,
        )
    };
    let dc_card = {
        let dc_handle = discord
            .and_then(|c| c.guild_name.clone())
            .unwrap_or_else(|| dc_fallback.clone());
        let connect = format!("{base_url}/admin/discord/install");
        let manage = format!("{base_url}/admin/discord");
        channel_card_html(
            &ChannelCardProps {
                key: "discord",
                name: &dc_name,
                connected: discord.is_some(),
                status_line: if discord.is_some() {
                    &dc_handle
                } else {
                    &dc_flavor
                },
                connect_href: &connect,
                manage_href: &manage,
            },
            locale,
        )
    };
    let integrations_section = format!(
        r#"<div class="card p-22">
    <h2>Integrations</h2>
    <p class="muted mb-16">Connect, reconfigure, or disconnect messaging channels.</p>
    <div class="channels-grid">{ig_card}{wa_card}{dc_card}</div>
  </div>"#
    );

    let google_row = if has_google {
        let unlink = if has_facebook {
            format!(
                "<button class=\"btn sm text-warn\"
                        style=\"border-color:var(--warn)\"
                        hx-delete=\"{base_url}/auth/unlink/google\"
                        hx-confirm=\"Unlink Google? You can still sign in with Facebook.\"
                        hx-target=\"{hash}linked-providers\" hx-swap=\"innerHTML\">Unlink</button>",
                base_url = base_url,
                hash = HASH,
            )
        } else {
            String::from("<span class=\"muted\">Only provider</span>")
        };
        format!(
            "<tr><td>Google</td><td>{email}</td><td>{unlink}</td></tr>",
            email = html_escape(&tenant.email),
            unlink = unlink,
        )
    } else {
        let redirect_uri = format!("{}/auth/callback", base_url);
        let link_url = format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=online&prompt=select_account",
            urlencoding::encode(google_client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode("openid email profile"),
        );
        if google_client_id.is_empty() {
            String::new()
        } else {
            format!(
                "<tr><td>Google</td><td class=\"muted\">Not linked</td><td><a href=\"{link_url}\" class=\"btn sm\">Link</a></td></tr>",
                link_url = html_escape(&link_url),
            )
        }
    };

    let facebook_row = if has_facebook {
        let unlink = if has_google {
            format!(
                "<button class=\"btn sm text-warn\"
                        style=\"border-color:var(--warn)\"
                        hx-delete=\"{base_url}/auth/unlink/facebook\"
                        hx-confirm=\"Unlink Facebook? You can still sign in with Google.\"
                        hx-target=\"{hash}linked-providers\" hx-swap=\"innerHTML\">Unlink</button>",
                base_url = base_url,
                hash = HASH,
            )
        } else {
            String::from("<span class=\"muted\">Only provider</span>")
        };
        format!(
            "<tr><td>Facebook</td><td>Connected</td><td>{unlink}</td></tr>",
            unlink = unlink,
        )
    } else {
        let fb_redirect_uri = format!("{}/auth/facebook/callback", base_url);
        let link_url = format!(
            "https://www.facebook.com/{}/dialog/oauth?client_id={}&redirect_uri={}&scope=email&response_type=code",
            crate::META_API_VERSION,
            urlencoding::encode(meta_app_id),
            urlencoding::encode(&fb_redirect_uri),
        );
        if meta_app_id.is_empty() {
            String::new()
        } else {
            format!(
                "<tr><td>Facebook</td><td class=\"muted\">Not linked</td><td><a href=\"{link_url}\" class=\"btn sm\">Link</a></td></tr>",
                link_url = html_escape(&link_url),
            )
        }
    };

    let conversation_section = conversation_settings_card(base_url, conversation, locale);

    let content = format!(
        "<div class=\"page-pad\">
        <h1 class=\"display-sm m-0 mb-16\">{h1}</h1>
        <div class=\"card p-22\">
            <h2>{linked_h2}</h2>
            <p class=\"muted mb-16\">{linked_lead}</p>
            <div id=\"linked-providers\" role=\"region\" aria-label=\"{linked_region}\">
                <div class=\"table-wrap\"><table>
                    <thead><tr><th scope=\"col\">{th_provider}</th><th scope=\"col\">{th_details}</th><th></th></tr></thead>
                    <tbody>{google_row}{facebook_row}</tbody>
                </table></div>
            </div>
        </div>
        {integrations_section}
        <div class=\"card p-22\" hx-ext=\"json-enc\">
            <h2>{currency_h2}</h2>
            <p class=\"muted mb-16\">{currency_lead}</p>
            <form hx-put=\"{base_url}/admin/settings/currency\" hx-target=\"{hash}currency-toast\" hx-swap=\"innerHTML\">
                <div class=\"row gap-12\">
                    <select class=\"select\" name=\"currency\" style=\"width:auto\">
                        <option value=\"INR\"{inr_sel}>{inr_label}</option>
                        <option value=\"USD\"{usd_sel}>{usd_label}</option>
                    </select>
                    <button type=\"submit\" class=\"btn sm\">{save}</button>
                </div>
            </form>
            <div id=\"currency-toast\" class=\"mt-8\" role=\"status\" aria-live=\"polite\" aria-atomic=\"true\"></div>
        </div>
        {conversation_section}
        <div class=\"card p-22\">
            <h2>{session_h2}</h2>
            <a href=\"{base_url}/auth/logout\" class=\"btn ghost\">{signout}</a>
        </div>
        <div class=\"card card-warn p-22\">
            <h2 class=\"text-warn\">{delete_h2}</h2>
            <p class=\"muted mb-16\">{delete_lead}</p>
            <button class=\"btn\" style=\"background:var(--warn);border-color:var(--warn);color:#fff\"
                    hx-delete=\"{base_url}/admin/delete-account\"
                    hx-confirm=\"{delete_confirm}\"
                    >{delete_cta}</button>
        </div>
        </div>",
        base_url = base_url,
        hash = HASH,
        google_row = google_row,
        facebook_row = facebook_row,
        integrations_section = integrations_section,
        conversation_section = conversation_section,
        inr_sel = if tenant.currency == crate::locale::Currency::Inr { " selected" } else { "" },
        usd_sel = if tenant.currency == crate::locale::Currency::Usd { " selected" } else { "" },
        h1 = t(locale, "admin-settings-h1"),
        linked_h2 = t(locale, "admin-settings-linked-h2"),
        linked_lead = t(locale, "admin-settings-linked-lead"),
        linked_region = html_escape(&t(locale, "admin-settings-linked-region")),
        th_provider = t(locale, "admin-settings-th-provider"),
        th_details = t(locale, "admin-settings-th-details"),
        currency_h2 = t(locale, "admin-settings-currency-h2"),
        currency_lead = t(locale, "admin-settings-currency-lead"),
        inr_label = t(locale, "admin-settings-currency-inr"),
        usd_label = t(locale, "admin-settings-currency-usd"),
        save = t(locale, "admin-save"),
        session_h2 = t(locale, "admin-settings-session-h2"),
        signout = t(locale, "admin-settings-signout"),
        delete_h2 = t(locale, "admin-settings-delete-h2"),
        delete_lead = t(locale, "admin-settings-delete-lead"),
        delete_confirm = html_escape(&t(locale, "admin-settings-delete-confirm")),
        delete_cta = t(locale, "admin-settings-delete-cta"),
    );

    let page = super::base::app_shell(&content, "Settings", base_url, locale);
    base_html(&t(locale, "admin-settings-title"), &page, locale)
}

pub fn admin_dashboard_html(
    whatsapp_accounts: &[WhatsAppAccount],
    instagram_accounts: &[InstagramAccount],
    lead_forms: &[LeadCaptureForm],
    billing: &TenantBilling,
    email_addrs: &[EmailAddress],
    base_url: &str,
    show_risk_gate_banner: bool,
    locale: &Locale,
) -> String {
    use super::base::app_shell;

    let has_email_domains = !email_addrs.is_empty();
    let suspended_banner = String::new();
    let risk_gate_banner = if show_risk_gate_banner {
        format!(
            r##"<div id="risk-gate-banner" class="card p-16 mb-16" style="border-left:4px solid var(--accent)">
  <div class="row gap-12" style="align-items:flex-start;justify-content:space-between">
    <div>
      <strong>{headline}</strong>
      <p class="muted fs-13 m-0 mt-4">{body}</p>
    </div>
    <button class="btn ghost sm"
      hx-post="/admin/risk-gate-banner/dismiss"
      hx-target="#risk-gate-banner"
      hx-swap="outerHTML">{dismiss}</button>
  </div>
</div>"##,
            headline = t(locale, "admin-dashboard-risk-banner-headline"),
            body = t(locale, "admin-dashboard-risk-banner-body"),
            dismiss = t(locale, "admin-dashboard-risk-banner-dismiss"),
        )
    } else {
        String::new()
    };

    // Sidebar: connected channels.
    let ig_label = html_escape(&t(locale, "admin-icon-instagram"));
    let wa_label = html_escape(&t(locale, "admin-icon-whatsapp"));
    let em_label = html_escape(&t(locale, "admin-icon-email"));
    let ig_icon = format!(
        r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-label="{ig_label}"><rect x="3" y="3" width="18" height="18" rx="5" stroke="currentColor" stroke-width="1.6"/><circle cx="12" cy="12" r="4.2" stroke="currentColor" stroke-width="1.6"/></svg>"#,
    );
    let wa_icon = format!(
        r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-label="{wa_label}"><path d="M4 20l1.3-4.1A8 8 0 1 1 8.2 18.8L4 20z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/></svg>"#,
    );
    let mail_icon = format!(
        r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-label="{em_label}"><rect x="3" y="5" width="18" height="14" rx="2" stroke="currentColor" stroke-width="1.6"/><path d="M3.5 6.5l8.5 6 8.5-6" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/></svg>"#,
    );

    let wa_text = t(locale, "admin-icon-whatsapp");
    let ig_text = t(locale, "admin-icon-instagram");
    let channel_rows: String = whatsapp_accounts
        .iter()
        .map(|a| {
            let dot = if a.auto_reply.enabled { r#"<span class="dot ok"></span>"# } else { r#"<span class="dot"></span>"# };
            format!(
                r#"<a href="{base_url}/admin/whatsapp/{id}" class="side-row"><span>{wa_icon}</span><div><div>{wa_text}</div><div class="mono muted">{phone}</div></div>{dot}</a>"#,
                base_url = base_url,
                id = html_escape(&a.id),
                phone = html_escape(&a.phone_number),
                wa_icon = wa_icon,
                wa_text = wa_text,
                dot = dot,
            )
        })
        .collect();

    let ig_rows: String = instagram_accounts
        .iter()
        .map(|a| {
            let dot = if a.enabled { r#"<span class="dot ok"></span>"# } else { r#"<span class="dot"></span>"# };
            format!(
                r#"<a href="{base_url}/admin/instagram/{id}" class="side-row"><span>{ig_icon}</span><div><div>{ig_text}</div><div class="mono muted">@{username}</div></div>{dot}</a>"#,
                base_url = base_url,
                id = html_escape(&a.id),
                username = html_escape(&a.instagram_username),
                ig_icon = ig_icon,
                ig_text = ig_text,
                dot = dot,
            )
        })
        .collect();

    let empty_hint = if whatsapp_accounts.is_empty() && instagram_accounts.is_empty() {
        format!(
            r#"<div class="side-row"><span class="muted fs-13">{prefix} <a href="/admin/whatsapp">{link}</a>.</span></div>"#,
            prefix = t(locale, "admin-side-empty-prefix"),
            link = t(locale, "admin-side-empty-link"),
        )
    } else {
        String::new()
    };

    let email_headline = if has_email_domains {
        t(locale, "admin-dashboard-email-headline-active")
    } else {
        t(locale, "admin-dashboard-email-headline-empty")
    };
    let email_cta = if has_email_domains {
        t(locale, "admin-dashboard-email-cta-active")
    } else {
        t(locale, "admin-dashboard-email-cta-empty")
    };
    let email_desc = if has_email_domains {
        t(locale, "admin-dashboard-email-desc-active")
    } else {
        t(locale, "admin-dashboard-email-desc-empty")
    };
    let content = format!(
        r#"<div class="dash-grid">
  <aside class="dash-side">
    <div class="card p-16">
      <div class="eyebrow">{side_channels}</div>
      <div class="side-list">
        {channel_rows}{ig_rows}{empty_hint}
        <a href="{base_url}/admin/email" class="side-row"><span>{mail_icon}</span><div><div>{email_row_name}</div><div class="mono muted">{email_row_cta}</div></div></a>
      </div>
    </div>
    <div class="card p-16 mt-14">
      <div class="eyebrow">{quick_links}</div>
      <div class="side-list">
        <a href="{base_url}/admin/lead-forms" class="side-row link-reset"><div class="flex-1 fs-13">{leads_prefix} ({lf_count})</div></a>
        <a href="{base_url}/admin/email/log" class="side-row link-reset"><div class="flex-1 fs-13">{email_log}</div></a>
      </div>
    </div>
  </aside>
  <main class="dash-main">
    {suspended_banner}
    {risk_gate_banner}
    <div class="card p-22">
      <div class="between mb-16">
        <div>
          <div class="eyebrow">{eyebrow}</div>
          <h3 class="display-sm m-0 mt-4">{headline}</h3>
        </div>
      </div>
      <div style="display:grid;grid-template-columns:repeat(4,1fr);gap:16px">
        <div class="card p-16 ta-center">
          <div class="stat-n serif">{wa_count}</div>
          <div class="mono muted fs-11">{stat_wa}</div>
        </div>
        <div class="card p-16 ta-center">
          <div class="stat-n serif">{ig_count}</div>
          <div class="mono muted fs-11">{stat_ig}</div>
        </div>
        <div class="card p-16 ta-center">
          <div class="stat-n serif">{lf_count}</div>
          <div class="mono muted fs-11">{stat_lf}</div>
        </div>
        <div class="card p-16 ta-center{credit_warn_cls}" style="{credit_warn_style}">
          <div class="stat-n serif">{credits}</div>
          <div class="mono muted fs-11">{stat_credits}</div>
        </div>
      </div>
    </div>
    <div class="card p-22 mt-16{email_highlight_cls}">
      <div class="between mb-12">
        <div>
          <div class="eyebrow">{email_eyebrow}</div>
          <h3 class="display-sm m-0 mt-4">{email_headline}</h3>
        </div>
        <a href="{base_url}/admin/email" class="btn sm">{email_cta}</a>
      </div>
      <p class="muted">{email_desc}</p>
    </div>
  </main>
</div>"#,
        base_url = base_url,
        channel_rows = channel_rows,
        ig_rows = ig_rows,
        empty_hint = empty_hint,
        mail_icon = mail_icon,
        suspended_banner = suspended_banner,
        wa_count = whatsapp_accounts.len(),
        ig_count = instagram_accounts.len(),
        lf_count = lead_forms.len(),
        credits = billing.total_remaining(),
        credit_warn_cls = if billing.total_remaining() <= 10 {
            " card-warn"
        } else {
            ""
        },
        credit_warn_style = if billing.total_remaining() <= 10 {
            "background:var(--accent-soft)"
        } else {
            ""
        },
        email_highlight_cls = if has_email_domains {
            ""
        } else {
            " card-accent"
        },
        email_headline = email_headline,
        email_cta = email_cta,
        email_desc = email_desc,
        risk_gate_banner = risk_gate_banner,
        side_channels = t(locale, "admin-side-channels"),
        email_row_name = t(locale, "admin-side-email-row-name"),
        email_row_cta = t(locale, "admin-side-email-row-cta"),
        quick_links = t(locale, "admin-side-quick-links"),
        leads_prefix = t(locale, "admin-side-lead-forms-prefix"),
        email_log = t(locale, "admin-side-email-log"),
        eyebrow = t(locale, "admin-dashboard-eyebrow"),
        headline = t(locale, "admin-dashboard-headline"),
        stat_wa = t(locale, "admin-dashboard-stat-whatsapp"),
        stat_ig = t(locale, "admin-dashboard-stat-instagram"),
        stat_lf = t(locale, "admin-dashboard-stat-leads"),
        stat_credits = t(locale, "admin-dashboard-stat-credits"),
        email_eyebrow = t(locale, "admin-dashboard-email-eyebrow"),
    );

    let page = app_shell(&content, "Overview", base_url, locale);
    base_html(&t(locale, "admin-dashboard-title"), &page, locale)
}

pub fn admin_whatsapp_list_html(
    accounts: &[WhatsAppAccount],
    base_url: &str,
    locale: &Locale,
) -> String {
    let yes_label = t(locale, "admin-yes");
    let no_label = t(locale, "admin-no");
    let edit_label = t(locale, "admin-edit");
    let delete_label = t(locale, "admin-delete");
    let delete_confirm = html_escape(&t(locale, "admin-wa-list-delete-confirm"));
    let rows: String = accounts
        .iter()
        .map(|a| {
            let enabled = if a.auto_reply.enabled {
                &yes_label
            } else {
                &no_label
            };
            format!(
                "<tr>
                    <td><a href=\"{base_url}/admin/whatsapp/{id}\">{name}</a></td>
                    <td><code>{phone}</code></td>
                    <td>{enabled}</td>
                    <td>
                        <a href=\"{base_url}/admin/whatsapp/{id}\" class=\"btn sm\">{edit}</a>
                        <button class=\"btn sm text-warn\" style=\"border-color:var(--warn)\"
                                hx-delete=\"{base_url}/admin/whatsapp/{id}\"
                                hx-confirm=\"{confirm}\"
                                hx-target=\"closest tr\" hx-swap=\"outerHTML\">{del}</button>
                    </td>
                </tr>",
                base_url = base_url,
                id = html_escape(&a.id),
                name = html_escape(&a.name),
                phone = html_escape(&a.phone_number),
                enabled = enabled,
                edit = edit_label,
                del = delete_label,
                confirm = delete_confirm,
            )
        })
        .collect();

    let empty = if accounts.is_empty() {
        format!(
            "<tr><td colspan=\"4\" class=\"muted\">{}</td></tr>",
            t(locale, "admin-wa-list-empty"),
        )
    } else {
        String::new()
    };

    let content = format!(
        "<div class=\"page-pad\">
        <div class=\"between mb-16\">
            <h1 class=\"display-sm m-0\">{h1}</h1>
            <a href=\"{base_url}/admin/whatsapp/new\" class=\"btn\">{add}</a>
        </div>
        <div id=\"toast\" role=\"status\" aria-live=\"polite\" aria-atomic=\"true\"></div>
        <div class=\"card p-22\">
            <div class=\"table-wrap\"><table>
                <thead><tr><th scope=\"col\">{th_name}</th><th scope=\"col\">{th_phone}</th><th scope=\"col\">{th_auto}</th><th></th></tr></thead>
                <tbody>{rows}{empty}</tbody>
            </table></div>
        </div>
        </div>",
        base_url = base_url,
        rows = rows,
        empty = empty,
        h1 = t(locale, "admin-wa-list-h1"),
        add = t(locale, "admin-wa-list-add"),
        th_name = t(locale, "admin-wa-list-th-name"),
        th_phone = t(locale, "admin-wa-list-th-phone"),
        th_auto = t(locale, "admin-wa-list-th-auto"),
    );

    let page = super::base::app_shell(&content, "Channels", base_url, locale);
    base_html(&t(locale, "admin-wa-list-title"), &page, locale)
}

pub fn admin_whatsapp_signup_html(
    base_url: &str,
    app_id: &str,
    config_id: &str,
    state: &str,
    locale: &Locale,
) -> String {
    let signup_back = t(locale, "admin-wa-edit-back");
    let signup_h1 = t(locale, "admin-wa-signup-h1");
    let signup_lead = t(locale, "admin-wa-signup-lead");
    let signup_cta = t(locale, "admin-wa-signup-cta");
    let signup_manual_prefix = t(locale, "admin-wa-signup-manual-prefix");
    let signup_manual_link = t(locale, "admin-wa-signup-manual-link");
    let signup_connecting = t(locale, "admin-wa-signup-connecting");
    let signup_error = t(locale, "admin-wa-signup-cancel-error");
    let content = format!(
        r#"<div class="page-pad">
        <p><a href="{base_url}/admin/whatsapp" class="btn ghost sm">{back}</a></p>
        <h1 class="display-sm" style="margin:8px 0 16px">{h1}</h1>
        <div class="card ta-center" style="padding: 2rem;">
            <p class="muted" style="margin-bottom: 1.5rem">{lead}</p>
            <div id="signup-error" class="text-warn mb-16" role="alert" aria-live="assertive"></div>
            <button id="signup-btn" class="btn" style="padding: 0.75rem 2rem; font-size: 1rem;">
                {cta}
            </button>
            <p class="muted" style="margin-top: 1.5rem; font-size: 0.85rem">
                {manual_prefix} <a href="{base_url}/admin/whatsapp/manual">{manual_link}</a>
            </p>
        </div>
        <script async defer crossorigin="anonymous" src="https://connect.facebook.net/en_US/sdk.js"></script>
        <script type="module" nonce="__CSP_NONCE__">
// Module scripts are deferred — handle the case where the SDK ran first
// (FB already exists) as well as the normal case (SDK still loading).
const fbReady = new Promise((resolve) => {{
  const init = () => {{
    FB.init({{
      appId: '{app_id}',
      autoLogAppEvents: true,
      xfbml: true,
      version: '{api_version}',
    }});
    resolve();
  }};
  if (typeof FB !== 'undefined' && typeof FB.init === 'function') init();
  else window.fbAsyncInit = init;
}});

const btn = document.getElementById('signup-btn');
const errDiv = document.getElementById('signup-error');
const SDK_TIMEOUT_MS = 8000;

btn?.addEventListener('click', async () => {{
  btn.disabled = true;
  btn.textContent = '{connecting}';
  errDiv.textContent = '';

  const fail = () => {{
    btn.disabled = false;
    btn.textContent = '{cta_again}';
    errDiv.textContent = '{cancel_error}';
  }};

  try {{
    const timeout = new Promise((_, reject) =>
      setTimeout(() => reject(new Error('sdk-timeout')), SDK_TIMEOUT_MS));
    await Promise.race([fbReady, timeout]);
  }} catch {{
    fail();
    return;
  }}

  const loginConfig = {{
    response_type: 'code',
    override_default_response_type: true,
    extras: {{ featureType: '', sessionInfoVersion: '3' }},
  }};
  const configId = '{config_id}';
  if (configId) loginConfig.config_id = configId;
  else loginConfig.scope = 'whatsapp_business_management,whatsapp_business_messaging';

  FB.login((response) => {{
    if (!response.authResponse?.code) {{
      fail();
      return;
    }}
    const form = document.createElement('form');
    form.method = 'POST';
    form.action = '{base_url}/whatsapp/signup/callback';
    const append = (name, value) => {{
      const input = document.createElement('input');
      input.type = 'hidden';
      input.name = name;
      input.value = value;
      form.appendChild(input);
    }};
    append('code', response.authResponse.code);
    append('state', '{state}');
    if (response.authResponse.phone_number_id) {{
      append('phone_number_id', response.authResponse.phone_number_id);
    }}
    document.body.appendChild(form);
    form.submit();
  }}, loginConfig);
}});
        </script>
        </div>"#,
        back = signup_back,
        h1 = signup_h1,
        lead = signup_lead,
        cta = signup_cta,
        manual_prefix = signup_manual_prefix,
        manual_link = signup_manual_link,
        connecting = signup_connecting,
        cta_again = signup_cta,
        cancel_error = signup_error,
        base_url = html_escape(base_url),
        app_id = html_escape(app_id),
        config_id = html_escape(config_id),
        state = html_escape(state),
        api_version = crate::META_API_VERSION,
    );

    let page = super::base::app_shell(&content, "Channels", base_url, locale);
    base_html(&t(locale, "admin-wa-signup-title"), &page, locale)
}

pub fn admin_whatsapp_edit_html(
    account: &WhatsAppAccount,
    base_url: &str,
    locale: &Locale,
) -> String {
    let canned = account.auto_reply.default_is_canned();
    let mode_static_sel = if canned { " selected" } else { "" };
    let mode_ai_sel = if !canned { " selected" } else { "" };
    let enabled_checked = if account.auto_reply.enabled {
        " checked"
    } else {
        ""
    };

    let wait_label_template = t(locale, "admin-wa-edit-wait-prefix");
    let content = format!(
        "<div class=\"page-pad\">
        <p><a href=\"{base_url}/admin/whatsapp\" class=\"btn ghost sm\">{back}</a></p>
        <h1 class=\"display-sm\" style=\"margin:8px 0 16px\">{h1}</h1>
        <div id=\"toast\" role=\"status\" aria-live=\"polite\" aria-atomic=\"true\"></div>
        <div class=\"card p-22\">
            <form hx-put=\"{base_url}/admin/whatsapp/{id}\" hx-target=\"{hash}toast\" hx-swap=\"innerHTML\">
                <div class=\"form-group\">
                    <label for=\"wa-name\">{lbl_name}</label>
                    <input type=\"text\" id=\"wa-name\" name=\"name\" value=\"{name}\" required>
                </div>
                <div class=\"form-group\">
                    <label for=\"wa-phone\">{lbl_phone}</label>
                    <input type=\"text\" id=\"wa-phone\" name=\"phone_number\" value=\"{phone}\" placeholder=\"{ph_phone}\" required>
                </div>
                <div class=\"form-group\">
                    <label for=\"wa-phone-id\">{lbl_phone_id}</label>
                    <input type=\"text\" id=\"wa-phone-id\" name=\"phone_number_id\" value=\"{phone_id}\" placeholder=\"{ph_phone_id}\" required class=\"mono\">
                </div>
                <h3 style=\"margin: 1rem 0 0.5rem;\">{h3_auto}</h3>
                <p class=\"muted fs-13 mb-12\">{rules_prefix} <a href=\"{base_url}/admin/rules/whatsapp/{id}\">{rules_link}</a>.</p>
                <div class=\"form-group\">
                    <label><input type=\"checkbox\" name=\"auto_reply_enabled\" value=\"true\"{enabled_checked}> {enabled}</label>
                </div>
                <div class=\"form-group\">
                    <label for=\"wa-mode\">{lbl_mode}</label>
                    <select id=\"wa-mode\" name=\"auto_reply_mode\">
                        <option value=\"canned\"{mode_static_sel}>{mode_static}</option>
                        <option value=\"prompt\"{mode_ai_sel}>{mode_ai}</option>
                    </select>
                </div>
                <div class=\"form-group\">
                    <label for=\"wa-prompt\">{lbl_prompt}</label>
                    <textarea id=\"wa-prompt\" name=\"auto_reply_prompt\" rows=\"3\">{prompt}</textarea>
                </div>
                <div class=\"form-group\">
                    <label for=\"wa-wait\">{wait_prefix} ({wait}s)</label>
                    <input id=\"wa-wait\" type=\"range\" min=\"0\" max=\"30\" step=\"1\" name=\"wait_seconds\" value=\"{wait}\" oninput=\"this.previousElementSibling.textContent='{wait_prefix} (' + this.value + 's)'\">
                    <small class=\"muted fs-12\">{wait_help}</small>
                </div>
                <div style=\"display: flex; justify-content: flex-end; gap: 0.5rem;\">
                    <button type=\"submit\" class=\"btn\">{save}</button>
                </div>
            </form>
        </div>
        </div>",
        base_url = base_url,
        id = html_escape(&account.id),
        name = html_escape(&account.name),
        phone = html_escape(&account.phone_number),
        phone_id = html_escape(&account.phone_number_id),
        prompt = html_escape(account.auto_reply.default_text()),
        enabled_checked = enabled_checked,
        mode_static_sel = mode_static_sel,
        mode_ai_sel = mode_ai_sel,
        hash = HASH,
        wait = account.auto_reply.wait_seconds,
        back = t(locale, "admin-wa-edit-back"),
        h1 = t(locale, "admin-wa-edit-h1"),
        lbl_name = t(locale, "admin-wa-edit-label-name"),
        lbl_phone = t(locale, "admin-wa-edit-label-phone"),
        lbl_phone_id = t(locale, "admin-wa-edit-label-phone-id"),
        ph_phone = t(locale, "admin-wa-edit-placeholder-phone"),
        ph_phone_id = t(locale, "admin-wa-edit-placeholder-phone-id"),
        h3_auto = t(locale, "admin-wa-edit-h3-auto"),
        rules_prefix = t(locale, "admin-wa-edit-rules-prefix"),
        rules_link = t(locale, "admin-wa-edit-rules-link"),
        enabled = t(locale, "admin-wa-edit-enabled"),
        lbl_mode = t(locale, "admin-wa-edit-mode"),
        mode_static = t(locale, "admin-wa-edit-mode-static"),
        mode_ai = t(locale, "admin-wa-edit-mode-ai"),
        lbl_prompt = t(locale, "admin-wa-edit-prompt"),
        wait_prefix = wait_label_template,
        wait_help = t(locale, "admin-wa-edit-wait-help"),
        save = t(locale, "admin-save"),
    );

    let page = super::base::app_shell(&content, "Channels", base_url, locale);
    base_html(&t(locale, "admin-wa-edit-title"), &page, locale)
}

pub fn admin_instagram_list_html(
    accounts: &[InstagramAccount],
    base_url: &str,
    tenant_id: &str,
    locale: &Locale,
) -> String {
    let rows: String = accounts
        .iter()
        .map(|a| {
            let status_label = if a.enabled {
                t(locale, "admin-active")
            } else {
                t(locale, "admin-disabled")
            };
            let auto_label = if a.auto_reply.enabled {
                t(locale, "admin-yes")
            } else {
                t(locale, "admin-no")
            };
            let edit_label = t(locale, "admin-edit");
            let remove_label = t(locale, "admin-remove");
            let confirm = html_escape(&t(locale, "admin-ig-list-delete-confirm"));
            format!(
                "<tr>
                    <td><a href=\"{base_url}/admin/instagram/{id}\">@{username}</a></td>
                    <td>{status}</td>
                    <td>{auto}</td>
                    <td>
                        <a href=\"{base_url}/admin/instagram/{id}\" class=\"btn sm\">{edit}</a>
                        <button class=\"btn sm text-warn\" style=\"border-color:var(--warn)\"
                                hx-delete=\"{base_url}/admin/instagram/{id}\"
                                hx-confirm=\"{confirm}\"
                                hx-target=\"closest tr\" hx-swap=\"outerHTML\">{remove}</button>
                    </td>
                </tr>",
                base_url = base_url,
                id = html_escape(&a.id),
                username = html_escape(&a.instagram_username),
                status = status_label,
                auto = auto_label,
                edit = edit_label,
                remove = remove_label,
                confirm = confirm,
            )
        })
        .collect();

    let empty = if accounts.is_empty() {
        format!(
            "<tr><td colspan=\"4\" class=\"muted\">{}</td></tr>",
            t(locale, "admin-ig-list-empty"),
        )
    } else {
        String::new()
    };

    let content = format!(
        "<div class=\"page-pad\">
        <div class=\"between mb-16\">
            <h1 class=\"display-sm m-0\">{h1}</h1>
            <a href=\"{base_url}/instagram/auth/{tenant_id}\" class=\"btn\">{add}</a>
        </div>
        <div id=\"toast\" role=\"status\" aria-live=\"polite\" aria-atomic=\"true\"></div>
        <div class=\"card p-22\">
            <div class=\"table-wrap\"><table>
                <thead><tr><th scope=\"col\">{th_user}</th><th scope=\"col\">{th_status}</th><th scope=\"col\">{th_auto}</th><th></th></tr></thead>
                <tbody>{rows}{empty}</tbody>
            </table></div>
        </div>
        </div>",
        base_url = base_url,
        tenant_id = html_escape(tenant_id),
        rows = rows,
        empty = empty,
        h1 = t(locale, "admin-ig-list-h1"),
        add = t(locale, "admin-ig-list-add"),
        th_user = t(locale, "admin-ig-list-th-username"),
        th_status = t(locale, "admin-ig-list-th-status"),
        th_auto = t(locale, "admin-ig-list-th-auto"),
    );

    let page = super::base::app_shell(&content, "Channels", base_url, locale);
    base_html(&t(locale, "admin-ig-list-title"), &page, locale)
}

pub fn admin_instagram_edit_html(
    account: &InstagramAccount,
    base_url: &str,
    locale: &Locale,
) -> String {
    let enabled_checked = if account.enabled { " checked" } else { "" };
    let ar_enabled_checked = if account.auto_reply.enabled {
        " checked"
    } else {
        ""
    };
    let canned = account.auto_reply.default_is_canned();
    let mode_static_sel = if canned { " selected" } else { "" };
    let mode_ai_sel = if !canned { " selected" } else { "" };

    let wait_prefix = t(locale, "admin-ig-edit-wait-prefix");
    let content = format!(
        "<div class=\"page-pad\">
        <p><a href=\"{base_url}/admin/instagram\" class=\"btn ghost sm\">{back}</a></p>
        <h1 class=\"display-sm\" style=\"margin:8px 0 4px\">{h1}</h1>
        <p class=\"muted mb-16\">@{username}</p>
        <div id=\"toast\" role=\"status\" aria-live=\"polite\" aria-atomic=\"true\"></div>
        <div class=\"card p-22\">
            <form hx-put=\"{base_url}/admin/instagram/{id}\" hx-target=\"{hash}toast\" hx-swap=\"innerHTML\">
                <div class=\"form-group\">
                    <label><input type=\"checkbox\" name=\"enabled\" value=\"true\"{enabled_checked}> {account_enabled}</label>
                </div>
                <h3 style=\"margin: 1rem 0 0.5rem;\">{h3_auto}</h3>
                <p class=\"muted fs-13 mb-12\">{rules_prefix} <a href=\"{base_url}/admin/rules/instagram/{id}\">{rules_link}</a>.</p>
                <div class=\"form-group\">
                    <label><input type=\"checkbox\" name=\"auto_reply_enabled\" value=\"true\"{ar_enabled_checked}> {enabled}</label>
                </div>
                <div class=\"form-group\">
                    <label for=\"ig-mode\">{lbl_mode}</label>
                    <select id=\"ig-mode\" name=\"auto_reply_mode\">
                        <option value=\"canned\"{mode_static_sel}>{mode_static}</option>
                        <option value=\"prompt\"{mode_ai_sel}>{mode_ai}</option>
                    </select>
                </div>
                <div class=\"form-group\">
                    <label for=\"ig-prompt\">{lbl_prompt}</label>
                    <textarea id=\"ig-prompt\" name=\"auto_reply_prompt\" rows=\"3\">{prompt}</textarea>
                </div>
                <div class=\"form-group\">
                    <label for=\"ig-wait\">{wait_prefix} ({wait}s)</label>
                    <input id=\"ig-wait\" type=\"range\" min=\"0\" max=\"30\" step=\"1\" name=\"wait_seconds\" value=\"{wait}\" oninput=\"this.previousElementSibling.textContent='{wait_prefix} (' + this.value + 's)'\">
                    <small class=\"muted fs-12\">{wait_help}</small>
                </div>
                <div style=\"display: flex; justify-content: flex-end;\">
                    <button type=\"submit\" class=\"btn\">{save}</button>
                </div>
            </form>
        </div>
        </div>",
        base_url = base_url,
        id = html_escape(&account.id),
        username = html_escape(&account.instagram_username),
        enabled_checked = enabled_checked,
        ar_enabled_checked = ar_enabled_checked,
        mode_static_sel = mode_static_sel,
        mode_ai_sel = mode_ai_sel,
        prompt = html_escape(account.auto_reply.default_text()),
        wait = account.auto_reply.wait_seconds,
        hash = HASH,
        back = t(locale, "admin-ig-edit-back"),
        h1 = t(locale, "admin-ig-edit-h1"),
        account_enabled = t(locale, "admin-ig-edit-account-enabled"),
        h3_auto = t(locale, "admin-ig-edit-h3-auto"),
        rules_prefix = t(locale, "admin-ig-edit-rules-prefix"),
        rules_link = t(locale, "admin-ig-edit-rules-link"),
        enabled = t(locale, "admin-ig-edit-enabled"),
        lbl_mode = t(locale, "admin-ig-edit-mode"),
        mode_static = t(locale, "admin-ig-edit-mode-static"),
        mode_ai = t(locale, "admin-ig-edit-mode-ai"),
        lbl_prompt = t(locale, "admin-ig-edit-prompt"),
        wait_prefix = wait_prefix,
        wait_help = t(locale, "admin-ig-edit-wait-help"),
        save = t(locale, "admin-save"),
    );

    let page = super::base::app_shell(&content, "Channels", base_url, locale);
    base_html(&t(locale, "admin-ig-edit-title"), &page, locale)
}

pub fn admin_lead_forms_list_html(
    forms: &[LeadCaptureForm],
    base_url: &str,
    locale: &Locale,
) -> String {
    let edit_label = t(locale, "admin-edit");
    let delete_label = t(locale, "admin-delete");
    let confirm = html_escape(&t(locale, "admin-lf-list-delete-confirm"));
    let rows: String = forms
        .iter()
        .map(|f| {
            let status = if f.enabled {
                t(locale, "admin-active")
            } else {
                t(locale, "admin-disabled")
            };
            format!(
                "<tr>
                    <td><a href=\"{base_url}/admin/lead-forms/{id}\">{name}</a></td>
                    <td><code>{slug}</code></td>
                    <td>{status}</td>
                    <td>
                        <a href=\"{base_url}/admin/lead-forms/{id}\" class=\"btn sm\">{edit}</a>
                        <button class=\"btn sm text-warn\" style=\"border-color:var(--warn)\"
                                hx-delete=\"{base_url}/admin/lead-forms/{id}\"
                                hx-confirm=\"{confirm}\"
                                hx-target=\"closest tr\" hx-swap=\"outerHTML\">{del}</button>
                    </td>
                </tr>",
                base_url = base_url,
                id = html_escape(&f.id),
                name = html_escape(&f.name),
                slug = html_escape(&f.slug),
                status = status,
                edit = edit_label,
                del = delete_label,
                confirm = confirm,
            )
        })
        .collect();

    let empty = if forms.is_empty() {
        format!(
            "<tr><td colspan=\"4\" class=\"muted\">{}</td></tr>",
            t(locale, "admin-lf-list-empty"),
        )
    } else {
        String::new()
    };

    let content = format!(
        "<p><a href=\"{base_url}/admin\">{back}</a></p>
        <div class=\"between mb-16\">
            <h1>{h1}</h1>
            <a href=\"{base_url}/admin/lead-forms/new\" class=\"btn\">{add}</a>
        </div>
        <div id=\"toast\" role=\"status\" aria-live=\"polite\" aria-atomic=\"true\"></div>
        <div class=\"card p-22\">
            <div class=\"table-wrap\"><table>
                <thead><tr><th scope=\"col\">{th_name}</th><th scope=\"col\">{th_slug}</th><th scope=\"col\">{th_status}</th><th></th></tr></thead>
                <tbody>{rows}{empty}</tbody>
            </table></div>
        </div>",
        base_url = base_url,
        rows = rows,
        empty = empty,
        back = t(locale, "admin-lf-list-back"),
        h1 = t(locale, "admin-lf-list-h1"),
        add = t(locale, "admin-lf-list-add"),
        th_name = t(locale, "admin-lf-list-th-name"),
        th_slug = t(locale, "admin-lf-list-th-slug"),
        th_status = t(locale, "admin-lf-list-th-status"),
    );

    base_html(&t(locale, "admin-lf-list-title"), &content, locale)
}

pub fn admin_lead_form_edit_html(
    form: &LeadCaptureForm,
    whatsapp_accounts: &[WhatsAppAccount],
    base_url: &str,
    locale: &Locale,
) -> String {
    let wa_options: String = whatsapp_accounts
        .iter()
        .map(|a| {
            let sel = if a.id == form.whatsapp_account_id {
                " selected"
            } else {
                ""
            };
            format!(
                "<option value=\"{id}\"{sel}>{name} ({phone})</option>",
                id = html_escape(&a.id),
                sel = sel,
                name = html_escape(&a.name),
                phone = html_escape(&a.phone_number),
            )
        })
        .collect();

    let canned = matches!(form.reply, ReplyResponse::Canned { .. });
    let mode_static_sel = if canned { " selected" } else { "" };
    let mode_ai_sel = if !canned { " selected" } else { "" };
    let reply_text = match &form.reply {
        ReplyResponse::Canned { text } | ReplyResponse::Prompt { text } => text.as_str(),
    };
    let enabled_checked = if form.enabled { " checked" } else { "" };
    let origins = form.allowed_origins.join("\n");

    let embed_code = format!(
        "&lt;iframe src=&quot;{base_url}/lead/{id}/{slug}&quot; width=&quot;400&quot; height=&quot;200&quot; frameborder=&quot;0&quot;&gt;&lt;/iframe&gt;",
        base_url = html_escape(base_url),
        id = html_escape(&form.id),
        slug = html_escape(&form.slug),
    );

    let content = format!(
        "<p><a href=\"{base_url}/admin/lead-forms\">{back}</a></p>
        <h1>{h1}</h1>
        <div id=\"toast\" role=\"status\" aria-live=\"polite\" aria-atomic=\"true\"></div>
        <div class=\"card p-22\">
            <form hx-put=\"{base_url}/admin/lead-forms/{id}\" hx-target=\"{hash}toast\" hx-swap=\"innerHTML\">
                <div class=\"form-group\">
                    <label for=\"lf-name\">{lbl_name}</label>
                    <input type=\"text\" id=\"lf-name\" name=\"name\" value=\"{name}\" required aria-required=\"true\">
                </div>
                <div class=\"form-group\">
                    <label><input type=\"checkbox\" name=\"enabled\" value=\"true\"{enabled_checked}> {lbl_enabled}</label>
                </div>
                <div class=\"form-group\">
                    <label for=\"lf-wa\">{lbl_wa}</label>
                    <select id=\"lf-wa\" name=\"whatsapp_account_id\" required aria-required=\"true\">{wa_options}</select>
                </div>
                <div class=\"form-group\">
                    <label for=\"lf-reply-mode\">{lbl_reply_mode}</label>
                    <select id=\"lf-reply-mode\" name=\"reply_mode\">
                        <option value=\"canned\"{mode_static_sel}>{mode_static}</option>
                        <option value=\"prompt\"{mode_ai_sel}>{mode_ai}</option>
                    </select>
                </div>
                <div class=\"form-group\">
                    <label for=\"lf-reply-prompt\">{lbl_reply_prompt}</label>
                    <textarea id=\"lf-reply-prompt\" name=\"reply_prompt\" rows=\"3\">{reply_prompt}</textarea>
                </div>
                <h3 style=\"margin: 1rem 0 0.5rem;\">{h3_style}</h3>
                <div class=\"form-group\">
                    <label for=\"lf-color-primary\">{lbl_primary}</label>
                    <input type=\"color\" id=\"lf-color-primary\" name=\"style_primary_color\" value=\"{s_primary}\">
                </div>
                <div class=\"form-group\">
                    <label for=\"lf-color-text\">{lbl_text}</label>
                    <input type=\"color\" id=\"lf-color-text\" name=\"style_text_color\" value=\"{s_text}\">
                </div>
                <div class=\"form-group\">
                    <label for=\"lf-color-bg\">{lbl_bg}</label>
                    <input type=\"color\" id=\"lf-color-bg\" name=\"style_background_color\" value=\"{s_bg}\">
                </div>
                <div class=\"form-group\">
                    <label for=\"lf-radius\">{lbl_radius}</label>
                    <input type=\"text\" id=\"lf-radius\" name=\"style_border_radius\" value=\"{s_radius}\">
                </div>
                <div class=\"form-group\">
                    <label for=\"lf-button-text\">{lbl_button}</label>
                    <input type=\"text\" id=\"lf-button-text\" name=\"style_button_text\" value=\"{s_button}\">
                </div>
                <div class=\"form-group\">
                    <label for=\"lf-placeholder\">{lbl_placeholder}</label>
                    <input type=\"text\" id=\"lf-placeholder\" name=\"style_placeholder_text\" value=\"{s_placeholder}\">
                </div>
                <div class=\"form-group\">
                    <label for=\"lf-success\">{lbl_success}</label>
                    <input type=\"text\" id=\"lf-success\" name=\"style_success_message\" value=\"{s_success}\">
                </div>
                <div class=\"form-group\">
                    <label for=\"lf-custom-css\">{lbl_css}</label>
                    <textarea id=\"lf-custom-css\" name=\"style_custom_css\" rows=\"3\">{s_css}</textarea>
                </div>
                <h3 style=\"margin: 1rem 0 0.5rem;\">{h3_origins}</h3>
                <div class=\"form-group\">
                    <label for=\"lf-origins\" class=\"sr-only\">{h3_origins}</label>
                    <textarea id=\"lf-origins\" name=\"allowed_origins\" rows=\"3\" placeholder=\"{ph_origins}\">{origins}</textarea>
                    <small class=\"muted\">{help_origins}</small>
                </div>
                <div style=\"display: flex; justify-content: flex-end;\">
                    <button type=\"submit\" class=\"btn\">{save}</button>
                </div>
            </form>
        </div>

        <div class=\"card p-22\">
            <h3>{embed_h3}</h3>
            <p class=\"muted mb-8\">{embed_lead}</p>
            <div class=\"row gap-8\">
                <code class=\"block flex-1\" style=\"padding: 0.5rem; overflow-x: auto; white-space: nowrap;\">{embed_code}</code>
                <button class=\"btn sm copy-btn\" data-copy-url=\"{embed_raw}\">{copy}</button>
            </div>
        </div>",
        base_url = base_url,
        id = html_escape(&form.id),
        name = html_escape(&form.name),
        enabled_checked = enabled_checked,
        wa_options = wa_options,
        mode_static_sel = mode_static_sel,
        mode_ai_sel = mode_ai_sel,
        reply_prompt = html_escape(reply_text),
        s_primary = html_escape(&form.style.primary_color),
        s_text = html_escape(&form.style.text_color),
        s_bg = html_escape(&form.style.background_color),
        s_radius = html_escape(&form.style.border_radius),
        s_button = html_escape(&form.style.button_text),
        s_placeholder = html_escape(&form.style.placeholder_text),
        s_success = html_escape(&form.style.success_message),
        s_css = html_escape(&form.style.custom_css),
        origins = html_escape(&origins),
        embed_code = embed_code,
        embed_raw = format!(
            "<iframe src=\"{base_url}/lead/{id}/{slug}\" width=\"400\" height=\"200\" frameborder=\"0\"></iframe>",
            base_url = base_url,
            id = form.id,
            slug = form.slug,
        ).replace('\'', "\\'"),
        hash = HASH,
        back = t(locale, "admin-lf-edit-back"),
        h1 = t(locale, "admin-lf-edit-h1"),
        lbl_name = t(locale, "admin-lf-edit-name"),
        lbl_enabled = t(locale, "admin-lf-edit-enabled"),
        lbl_wa = t(locale, "admin-lf-edit-wa-account"),
        lbl_reply_mode = t(locale, "admin-lf-edit-reply-mode"),
        mode_static = t(locale, "admin-lf-edit-reply-mode-static"),
        mode_ai = t(locale, "admin-lf-edit-reply-mode-ai"),
        lbl_reply_prompt = t(locale, "admin-lf-edit-reply-prompt"),
        h3_style = t(locale, "admin-lf-edit-style-h3"),
        lbl_primary = t(locale, "admin-lf-edit-style-primary"),
        lbl_text = t(locale, "admin-lf-edit-style-text"),
        lbl_bg = t(locale, "admin-lf-edit-style-bg"),
        lbl_radius = t(locale, "admin-lf-edit-style-radius"),
        lbl_button = t(locale, "admin-lf-edit-style-button"),
        lbl_placeholder = t(locale, "admin-lf-edit-style-placeholder"),
        lbl_success = t(locale, "admin-lf-edit-style-success"),
        lbl_css = t(locale, "admin-lf-edit-style-css"),
        h3_origins = t(locale, "admin-lf-edit-origins-h3"),
        ph_origins = html_escape(&t(locale, "admin-lf-edit-origins-placeholder")),
        help_origins = t(locale, "admin-lf-edit-origins-help"),
        save = t(locale, "admin-lf-edit-save"),
        embed_h3 = t(locale, "admin-lf-edit-embed-h3"),
        embed_lead = t(locale, "admin-lf-edit-embed-lead"),
        copy = t(locale, "admin-lf-edit-embed-copy"),
    );

    base_html(&t(locale, "admin-lf-edit-title"), &content, locale)
}

pub fn admin_success_html(message: &str) -> String {
    format!(
        "<div class=\"success\" role=\"status\">{}</div>",
        html_escape(message)
    )
}

#[allow(dead_code)]
pub fn admin_error_html(message: &str) -> String {
    format!(
        "<div class=\"error\" role=\"alert\">{}</div>",
        html_escape(message)
    )
}

#[cfg(test)]
mod conversation_card_tests {
    use super::*;

    #[test]
    fn empty_config_shows_default_placeholders_and_blank_values() {
        let cfg = ConversationConfig::default();
        let html = conversation_settings_card("https://example.test", &cfg, &Locale::default_inr());

        // Form posts to the right endpoint with the right swap target.
        assert!(
            html.contains(r#"hx-put="https://example.test/admin/settings/conversation""#),
            "form action missing"
        );
        assert!(html.contains(r##"hx-target="#conversation-toast""##));

        // Each field is present with its `name=` matching ConversationConfig.
        assert!(html.contains(r#"name="idle_gap_mins""#));
        assert!(html.contains(r#"name="handoff_cooldown_mins""#));
        assert!(html.contains(r#"name="max_history_messages""#));

        // No values pre-filled when the config is empty.
        assert!(html.contains(r#"value="""#));

        // Defaults appear as placeholders so tenants see what they'll
        // fall back to without saving.
        assert!(html.contains(&format!(
            "placeholder=\"{}\"",
            crate::prompt::DEFAULT_CONVERSATION_IDLE_GAP_MINS
        )));
        assert!(html.contains(&format!(
            "placeholder=\"{}\"",
            crate::prompt::DEFAULT_HANDOFF_COOLDOWN_MINS
        )));
        assert!(html.contains(&format!(
            "placeholder=\"{}\"",
            crate::prompt::DEFAULT_CONVERSATION_MAX_MESSAGES
        )));

        // Bounds are wired into the inputs so the browser catches
        // out-of-range values before submit.
        assert!(html.contains(r#"min="5""#));
        assert!(html.contains(r#"max="1440""#));
        assert!(html.contains(r#"min="1""#));
        assert!(html.contains(r#"max="200""#));
    }

    #[test]
    fn populated_config_renders_existing_values() {
        let cfg = ConversationConfig {
            idle_gap_mins: Some(120),
            handoff_cooldown_mins: Some(45),
            max_history_messages: Some(12),
        };
        let html = conversation_settings_card("https://example.test", &cfg, &Locale::default_inr());

        assert!(html.contains(r#"name="idle_gap_mins""#) && html.contains(r#"value="120""#));
        assert!(html.contains(r#"name="handoff_cooldown_mins""#) && html.contains(r#"value="45""#));
        assert!(html.contains(r#"name="max_history_messages""#) && html.contains(r#"value="12""#));
    }

    #[test]
    fn aria_live_toast_div_is_present_for_inline_save_feedback() {
        // The PUT handler returns a fragment that HTMX swaps into
        // this div. Without it, success/error messages would land on
        // a fresh page load instead of inline.
        let html = conversation_settings_card(
            "https://example.test",
            &ConversationConfig::default(),
            &Locale::default_inr(),
        );
        assert!(
            html.contains(r#"id="conversation-toast""#) && html.contains(r#"aria-live="polite""#)
        );
    }
}
