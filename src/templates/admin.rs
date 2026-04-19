//! Admin dashboard templates for messaging-first product

use crate::helpers::html_escape;
use crate::types::*;

use super::base::base_html;
use super::HASH;

pub fn auth_login_html(
    base_url: &str,
    google_client_id: &str,
    meta_app_id: &str,
    last_provider: Option<&str>,
) -> String {
    let redirect_uri = format!("{}/auth/callback", base_url);
    let google_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=online&prompt=select_account",
        urlencoding::encode(google_client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("openid email profile"),
    );
    let fb_url = format!("{}/auth/facebook", base_url);

    let google_svg = r##"<svg width="18" height="18" viewBox="0 0 18 18" xmlns="http://www.w3.org/2000/svg"><path d="M17.64 9.2c0-.637-.057-1.251-.164-1.84H9v3.481h4.844a4.14 4.14 0 01-1.796 2.716v2.259h2.908c1.702-1.567 2.684-3.875 2.684-6.615z" fill="#4285F4"/><path d="M9 18c2.43 0 4.467-.806 5.956-2.18l-2.908-2.259c-.806.54-1.837.86-3.048.86-2.344 0-4.328-1.584-5.036-3.711H.957v2.332A8.997 8.997 0 009 18z" fill="#34A853"/><path d="M3.964 10.71A5.41 5.41 0 013.682 9c0-.593.102-1.17.282-1.71V4.958H.957A8.996 8.996 0 000 9c0 1.452.348 2.827.957 4.042l3.007-2.332z" fill="#FBBC05"/><path d="M9 3.58c1.321 0 2.508.454 3.44 1.345l2.582-2.58C13.463.891 11.426 0 9 0A8.997 8.997 0 00.957 4.958L3.964 7.29C4.672 5.163 6.656 3.58 9 3.58z" fill="#EA4335"/></svg>"##;
    let fb_svg = r##"<svg width="18" height="18" viewBox="0 0 24 24" fill="#1877F2"><path d="M24 12.073c0-6.627-5.373-12-12-12s-12 5.373-12 12c0 5.99 4.388 10.954 10.125 11.854v-8.385H7.078v-3.47h3.047V9.43c0-3.007 1.792-4.669 4.533-4.669 1.312 0 2.686.235 2.686.235v2.953H15.83c-1.491 0-1.956.925-1.956 1.874v2.25h3.328l-.532 3.47h-2.796v8.385C19.612 23.027 24 18.062 24 12.073z"/></svg>"##;

    let (primary_btn, secondary_btn) = if last_provider == Some("facebook") {
        (
            format!(
                r#"<a href="{fb_url}" class="btn primary lg" style="width:100%;justify-content:center">{fb_svg} Continue with Facebook</a>"#,
            ),
            format!(
                r#"<a href="{google_url}" class="btn ghost" style="width:100%;justify-content:center">{google_svg} Sign in with Google</a>"#,
                google_url = html_escape(&google_url),
            ),
        )
    } else {
        (
            format!(
                r#"<a href="{google_url}" class="btn primary lg" style="width:100%;justify-content:center">{google_svg} Sign in with Google</a>"#,
                google_url = html_escape(&google_url),
            ),
            format!(
                r#"<a href="{fb_url}" class="btn ghost" style="width:100%;justify-content:center">{fb_svg} Sign in with Facebook</a>"#,
            ),
        )
    };

    let content = format!(
        r#"<div style="max-width:360px;margin:4rem auto;text-align:center">
    <div style="margin-bottom:2rem">{logo}
    <div class="serif" style="font-size:28px;margin-top:8px">Concierge</div></div>
    <p class="muted" style="margin-bottom:2rem">Sign in to manage your messaging channels.</p>
    <div class="stack gap-12">
      {primary_btn}
      {secondary_btn}
    </div>
    <a href="/" class="btn ghost sm" style="margin-top:24px">&larr; Back to home</a>
</div>"#,
        logo = super::base::LOGO_INLINE,
        primary_btn = primary_btn,
        secondary_btn = secondary_btn,
    );

    base_html("Sign In - Concierge", &content)
}

pub fn admin_settings_html(
    tenant: &Tenant,
    base_url: &str,
    google_client_id: &str,
    meta_app_id: &str,
) -> String {
    let has_google = !tenant.email.is_empty();
    let has_facebook = tenant.facebook_id.is_some();

    let google_row = if has_google {
        let unlink = if has_facebook {
            format!(
                "<button class=\"btn sm\"
                        style=\"border-color:var(--warn);color:var(--warn)\"
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
                "<button class=\"btn sm\"
                        style=\"border-color:var(--warn);color:var(--warn)\"
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

    let content = format!(
        "<p><a href=\"{base_url}/admin\">&larr; Back to Dashboard</a></p>
        <h1>Settings</h1>
        <div class=\"card\" style=\"padding:22px\">
            <h2>Linked Accounts</h2>
            <p style=\"margin-bottom: 1rem;\" class=\"muted\">Sign-in providers connected to your account.</p>
            <div id=\"linked-providers\" role=\"region\" aria-label=\"Linked accounts\">
                <div class=\"table-wrap\"><table>
                    <thead><tr><th scope=\"col\">Provider</th><th scope=\"col\">Details</th><th></th></tr></thead>
                    <tbody>{google_row}{facebook_row}</tbody>
                </table></div>
            </div>
        </div>
        <div class=\"card\" style=\"padding:22px\">
            <h2>Session</h2>
            <a href=\"{base_url}/auth/logout\" class=\"btn ghost\">Sign Out</a>
        </div>
        <div class=\"card\" style=\"padding:22px;border-color:var(--warn)\">
            <h2 style=\"color:var(--warn)\">Delete Account</h2>
            <p style=\"margin-bottom: 1rem;\" class=\"muted\">Permanently delete your account and all associated data. This cannot be undone.</p>
            <button class=\"btn\" style=\"background:var(--warn);border-color:var(--warn);color:#fff\"
                    hx-delete=\"{base_url}/admin/delete-account\"
                    hx-confirm=\"Are you sure? This will permanently delete your account and ALL data. This cannot be undone.\"
                    >Delete My Account</button>
        </div>",
        base_url = base_url,
        google_row = google_row,
        facebook_row = facebook_row,
    );

    base_html("Settings - Concierge", &content)
}

pub fn admin_dashboard_html(
    whatsapp_accounts: &[WhatsAppAccount],
    instagram_accounts: &[InstagramAccount],
    lead_forms: &[LeadCaptureForm],
    billing: &TenantBilling,
    base_url: &str,
) -> String {
    use super::base::app_shell;
    use super::base::LOGO_INLINE;

    // Sidebar: connected channels
    let ig_icon = r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-label="Instagram"><rect x="3" y="3" width="18" height="18" rx="5" stroke="currentColor" stroke-width="1.6"/><circle cx="12" cy="12" r="4.2" stroke="currentColor" stroke-width="1.6"/></svg>"#;
    let wa_icon = r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-label="WhatsApp"><path d="M4 20l1.3-4.1A8 8 0 1 1 8.2 18.8L4 20z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/></svg>"#;
    let mail_icon = r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-label="Email"><rect x="3" y="5" width="18" height="14" rx="2" stroke="currentColor" stroke-width="1.6"/><path d="M3.5 6.5l8.5 6 8.5-6" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/></svg>"#;

    let channel_rows: String = whatsapp_accounts
        .iter()
        .map(|a| {
            let dot = if a.auto_reply.enabled { r#"<span class="dot ok"></span>"# } else { r#"<span class="dot"></span>"# };
            format!(
                r#"<a href="{base_url}/admin/whatsapp/{id}" class="side-row"><span>{wa_icon}</span><div><div>WhatsApp</div><div class="mono muted">{phone}</div></div>{dot}</a>"#,
                base_url = base_url,
                id = html_escape(&a.id),
                phone = html_escape(&a.phone_number),
                wa_icon = wa_icon,
                dot = dot,
            )
        })
        .collect();

    let ig_rows: String = instagram_accounts
        .iter()
        .map(|a| {
            let dot = if a.enabled { r#"<span class="dot ok"></span>"# } else { r#"<span class="dot"></span>"# };
            format!(
                r#"<a href="{base_url}/admin/instagram/{id}" class="side-row"><span>{ig_icon}</span><div><div>Instagram</div><div class="mono muted">@{username}</div></div>{dot}</a>"#,
                base_url = base_url,
                id = html_escape(&a.id),
                username = html_escape(&a.instagram_username),
                ig_icon = ig_icon,
                dot = dot,
            )
        })
        .collect();

    let empty_hint = if whatsapp_accounts.is_empty() && instagram_accounts.is_empty() {
        r#"<div class="side-row"><span class="muted" style="font-size:13px">No channels connected yet. <a href="/admin/whatsapp">Add one</a>.</span></div>"#
    } else {
        ""
    };

    let content = format!(
        r#"<div class="dash-grid">
  <aside class="dash-side">
    <div class="card" style="padding:16px">
      <div class="eyebrow">Connected channels</div>
      <div class="side-list">
        {channel_rows}{ig_rows}{empty_hint}
        <a href="{base_url}/admin/email" class="side-row"><span>{mail_icon}</span><div><div>Email Routing</div><div class="mono muted">Configure rules</div></div></a>
      </div>
    </div>
    <div class="card" style="padding:16px;margin-top:14px">
      <div class="eyebrow">Quick links</div>
      <div class="side-list">
        <a href="{base_url}/admin/lead-forms" class="side-row" style="text-decoration:none;color:inherit"><div style="flex:1;font-size:13px">Lead Forms ({lf_count})</div></a>
        <a href="{base_url}/admin/email/log" class="side-row" style="text-decoration:none;color:inherit"><div style="flex:1;font-size:13px">Email Log</div></a>
        <a href="{base_url}/admin/wizard" class="side-row" style="text-decoration:none;color:inherit"><div style="flex:1;font-size:13px">Replay Onboarding</div></a>
      </div>
    </div>
  </aside>
  <main class="dash-main">
    <div class="card" style="padding:22px">
      <div class="between" style="margin-bottom:16px">
        <div>
          <div class="eyebrow">Overview</div>
          <h3 class="display-sm" style="margin:4px 0 0">Your concierge is on duty.</h3>
        </div>
      </div>
      <div style="display:grid;grid-template-columns:repeat(4,1fr);gap:16px">
        <div class="card" style="padding:16px;text-align:center">
          <div class="stat-n serif">{wa_count}</div>
          <div class="mono muted" style="font-size:11px">WhatsApp</div>
        </div>
        <div class="card" style="padding:16px;text-align:center">
          <div class="stat-n serif">{ig_count}</div>
          <div class="mono muted" style="font-size:11px">Instagram</div>
        </div>
        <div class="card" style="padding:16px;text-align:center">
          <div class="stat-n serif">{lf_count}</div>
          <div class="mono muted" style="font-size:11px">Lead Forms</div>
        </div>
        <div class="card" style="padding:16px;text-align:center{credit_warn_style}">
          <div class="stat-n serif">{credits}</div>
          <div class="mono muted" style="font-size:11px">Reply Credits</div>
        </div>
      </div>
    </div>
    <div class="card" style="padding:22px;margin-top:16px">
      <div class="between" style="margin-bottom:12px">
        <div>
          <div class="eyebrow">Email Routing</div>
          <h3 class="display-sm" style="margin:4px 0 0">Rules for the mail that comes in.</h3>
        </div>
        <a href="{base_url}/admin/email" class="btn sm">Manage rules</a>
      </div>
      <p class="muted">Configure domains and routing rules to forward, drop, or AI-reply to incoming email.</p>
    </div>
  </main>
</div>"#,
        base_url = base_url,
        channel_rows = channel_rows,
        ig_rows = ig_rows,
        empty_hint = empty_hint,
        mail_icon = mail_icon,
        wa_count = whatsapp_accounts.len(),
        ig_count = instagram_accounts.len(),
        lf_count = lead_forms.len(),
        credits = billing.total_remaining(),
        credit_warn_style = if billing.total_remaining() <= 10 {
            ";border-color:var(--warn);background:var(--accent-soft)"
        } else {
            ""
        },
    );

    let page = app_shell(&content, "Overview", base_url);
    base_html("Dashboard - Concierge", &page)
}

pub fn admin_whatsapp_list_html(accounts: &[WhatsAppAccount], base_url: &str) -> String {
    let rows: String = accounts
        .iter()
        .map(|a| {
            let enabled = if a.auto_reply.enabled { "Yes" } else { "No" };
            format!(
                "<tr>
                    <td><a href=\"{base_url}/admin/whatsapp/{id}\">{name}</a></td>
                    <td><code>{phone}</code></td>
                    <td>{enabled}</td>
                    <td>
                        <a href=\"{base_url}/admin/whatsapp/{id}\" class=\"btn sm\">Edit</a>
                        <button class=\"btn sm\" style=\"border-color:var(--warn);color:var(--warn)\"
                                hx-delete=\"{base_url}/admin/whatsapp/{id}\"
                                hx-confirm=\"Delete this WhatsApp account?\"
                                hx-target=\"closest tr\" hx-swap=\"outerHTML\">Delete</button>
                    </td>
                </tr>",
                base_url = base_url,
                id = html_escape(&a.id),
                name = html_escape(&a.name),
                phone = html_escape(&a.phone_number),
                enabled = enabled,
            )
        })
        .collect();

    let empty = if accounts.is_empty() {
        "<tr><td colspan=\"4\" class=\"muted\">No WhatsApp accounts configured.</td></tr>"
    } else {
        ""
    };

    let content = format!(
        "<p><a href=\"{base_url}/admin\">&larr; Back to Dashboard</a></p>
        <div style=\"display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;\">
            <h1>WhatsApp Accounts</h1>
            <a href=\"{base_url}/admin/whatsapp/new\" class=\"btn\">+ Connect WhatsApp Number</a>
        </div>
        <div id=\"toast\"></div>
        <div class=\"card\" style=\"padding:22px\">
            <div class=\"table-wrap\"><table>
                <thead><tr><th scope=\"col\">Name</th><th scope=\"col\">Phone</th><th scope=\"col\">Auto-Reply</th><th></th></tr></thead>
                <tbody>{rows}{empty}</tbody>
            </table></div>
        </div>",
        base_url = base_url,
        rows = rows,
        empty = empty,
    );

    base_html("WhatsApp Accounts - Concierge", &content)
}

pub fn admin_whatsapp_signup_html(
    base_url: &str,
    app_id: &str,
    config_id: &str,
    state: &str,
) -> String {
    let content = format!(
        r#"<p><a href="{base_url}/admin/whatsapp">&larr; Back to WhatsApp Accounts</a></p>
        <h1>Connect WhatsApp Number</h1>
        <div class="card" style="text-align: center; padding: 2rem;">
            <p class="muted" style="margin-bottom: 1.5rem;">Click the button below to register your phone number through Meta's WhatsApp setup flow.</p>
            <div id="signup-error" style="color: var(--warn); margin-bottom: 1rem;"></div>
            <button id="signup-btn" class="btn" style="padding: 0.75rem 2rem; font-size: 1rem;" onclick="launchSignup()">
                Connect WhatsApp Number
            </button>
            <p class="muted" style="margin-top: 1.5rem; font-size: 0.85rem;">
                Or <a href="{base_url}/admin/whatsapp/manual">enter phone number ID manually</a>
            </p>
        </div>
        <script async defer crossorigin="anonymous" src="https://connect.facebook.net/en_US/sdk.js"></script>
        <script>
            window.fbAsyncInit = function() {{
                FB.init({{
                    appId: '{app_id}',
                    autoLogAppEvents: true,
                    xfbml: true,
                    version: '{api_version}'
                }});
            }};

            function launchSignup() {{
                var btn = document.getElementById('signup-btn');
                var errDiv = document.getElementById('signup-error');
                btn.disabled = true;
                btn.textContent = 'Connecting...';
                errDiv.textContent = '';

                var loginConfig = {{
                    response_type: 'code',
                    override_default_response_type: true,
                    extras: {{
                        featureType: '',
                        sessionInfoVersion: '3'
                    }}
                }};

                var configId = '{config_id}';
                if (configId) {{
                    loginConfig.config_id = configId;
                }} else {{
                    loginConfig.scope = 'whatsapp_business_management,whatsapp_business_messaging';
                }}

                FB.login(function(response) {{
                    if (response.authResponse && response.authResponse.code) {{
                        var form = document.createElement('form');
                        form.method = 'POST';
                        form.action = '{base_url}/whatsapp/signup/callback';

                        var codeInput = document.createElement('input');
                        codeInput.type = 'hidden';
                        codeInput.name = 'code';
                        codeInput.value = response.authResponse.code;
                        form.appendChild(codeInput);

                        var stateInput = document.createElement('input');
                        stateInput.type = 'hidden';
                        stateInput.name = 'state';
                        stateInput.value = '{state}';
                        form.appendChild(stateInput);

                        // If phone_number_id is in the response, send it too
                        if (response.authResponse.phone_number_id) {{
                            var phoneInput = document.createElement('input');
                            phoneInput.type = 'hidden';
                            phoneInput.name = 'phone_number_id';
                            phoneInput.value = response.authResponse.phone_number_id;
                            form.appendChild(phoneInput);
                        }}

                        document.body.appendChild(form);
                        form.submit();
                    }} else {{
                        btn.disabled = false;
                        btn.textContent = 'Connect WhatsApp Number';
                        errDiv.textContent = 'Signup was cancelled or failed. Please try again.';
                    }}
                }}, loginConfig);
            }}
        </script>"#,
        base_url = html_escape(base_url),
        app_id = html_escape(app_id),
        config_id = html_escape(config_id),
        state = html_escape(state),
        api_version = crate::META_API_VERSION,
    );

    base_html("Connect WhatsApp - Concierge", &content)
}

pub fn admin_whatsapp_edit_html(account: &WhatsAppAccount, base_url: &str) -> String {
    let mode_static_sel = if account.auto_reply.mode == AutoReplyMode::Static {
        " selected"
    } else {
        ""
    };
    let mode_ai_sel = if account.auto_reply.mode == AutoReplyMode::Ai {
        " selected"
    } else {
        ""
    };
    let enabled_checked = if account.auto_reply.enabled {
        " checked"
    } else {
        ""
    };

    let content = format!(
        "<p><a href=\"{base_url}/admin/whatsapp\">&larr; Back to WhatsApp Accounts</a></p>
        <h1>Edit WhatsApp Account</h1>
        <div id=\"toast\"></div>
        <div class=\"card\" style=\"padding:22px\">
            <form hx-put=\"{base_url}/admin/whatsapp/{id}\" hx-target=\"{hash}toast\" hx-swap=\"innerHTML\">
                <div class=\"form-group\">
                    <label for=\"wa-name\">Name</label>
                    <input type=\"text\" id=\"wa-name\" name=\"name\" value=\"{name}\" required>
                </div>
                <div class=\"form-group\">
                    <label for=\"wa-phone\">Phone Number</label>
                    <input type=\"text\" id=\"wa-phone\" name=\"phone_number\" value=\"{phone}\" placeholder=\"+1234567890\" required>
                </div>
                <div class=\"form-group\">
                    <label for=\"wa-phone-id\">Phone Number ID</label>
                    <input type=\"text\" id=\"wa-phone-id\" name=\"phone_number_id\" value=\"{phone_id}\" placeholder=\"Meta phone number ID\" required style=\"font-family: monospace;\">
                </div>
                <h3 style=\"margin: 1rem 0 0.5rem;\">Auto-Reply</h3>
                <div class=\"form-group\">
                    <label><input type=\"checkbox\" name=\"auto_reply_enabled\" value=\"true\"{enabled_checked}> Enabled</label>
                </div>
                <div class=\"form-group\">
                    <label for=\"wa-mode\">Mode</label>
                    <select id=\"wa-mode\" name=\"auto_reply_mode\">
                        <option value=\"Static\"{mode_static_sel}>Static</option>
                        <option value=\"Ai\"{mode_ai_sel}>AI</option>
                    </select>
                </div>
                <div class=\"form-group\">
                    <label for=\"wa-prompt\">Prompt / Message</label>
                    <textarea id=\"wa-prompt\" name=\"auto_reply_prompt\" rows=\"3\">{prompt}</textarea>
                </div>
                <div style=\"display: flex; justify-content: flex-end; gap: 0.5rem;\">
                    <button type=\"submit\" class=\"btn\">Save</button>
                </div>
            </form>
        </div>",
        base_url = base_url,
        id = html_escape(&account.id),
        name = html_escape(&account.name),
        phone = html_escape(&account.phone_number),
        phone_id = html_escape(&account.phone_number_id),
        prompt = html_escape(&account.auto_reply.prompt),
        enabled_checked = enabled_checked,
        mode_static_sel = mode_static_sel,
        mode_ai_sel = mode_ai_sel,
        hash = HASH,
    );

    base_html("Edit WhatsApp Account - Concierge", &content)
}

pub fn admin_instagram_list_html(
    accounts: &[InstagramAccount],
    base_url: &str,
    tenant_id: &str,
) -> String {
    let rows: String = accounts
        .iter()
        .map(|a| {
            let status = if a.enabled { "Active" } else { "Disabled" };
            let auto = if a.auto_reply.enabled { "Yes" } else { "No" };
            format!(
                "<tr>
                    <td><a href=\"{base_url}/admin/instagram/{id}\">@{username}</a></td>
                    <td>{status}</td>
                    <td>{auto}</td>
                    <td>
                        <a href=\"{base_url}/admin/instagram/{id}\" class=\"btn sm\">Edit</a>
                        <button class=\"btn sm\" style=\"border-color:var(--warn);color:var(--warn)\"
                                hx-delete=\"{base_url}/admin/instagram/{id}\"
                                hx-confirm=\"Remove this Instagram account?\"
                                hx-target=\"closest tr\" hx-swap=\"outerHTML\">Remove</button>
                    </td>
                </tr>",
                base_url = base_url,
                id = html_escape(&a.id),
                username = html_escape(&a.instagram_username),
                status = status,
                auto = auto,
            )
        })
        .collect();

    let empty = if accounts.is_empty() {
        "<tr><td colspan=\"4\" class=\"muted\">No Instagram accounts connected.</td></tr>"
    } else {
        ""
    };

    let content = format!(
        "<p><a href=\"{base_url}/admin\">&larr; Back to Dashboard</a></p>
        <div style=\"display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;\">
            <h1>Instagram Accounts</h1>
            <a href=\"{base_url}/instagram/auth/{tenant_id}\" class=\"btn\">+ Connect Account</a>
        </div>
        <div id=\"toast\"></div>
        <div class=\"card\" style=\"padding:22px\">
            <div class=\"table-wrap\"><table>
                <thead><tr><th scope=\"col\">Username</th><th scope=\"col\">Status</th><th scope=\"col\">Auto-Reply</th><th></th></tr></thead>
                <tbody>{rows}{empty}</tbody>
            </table></div>
        </div>",
        base_url = base_url,
        tenant_id = html_escape(tenant_id),
        rows = rows,
        empty = empty,
    );

    base_html("Instagram Accounts - Concierge", &content)
}

pub fn admin_instagram_edit_html(account: &InstagramAccount, base_url: &str) -> String {
    let enabled_checked = if account.enabled { " checked" } else { "" };
    let ar_enabled_checked = if account.auto_reply.enabled {
        " checked"
    } else {
        ""
    };
    let mode_static_sel = if account.auto_reply.mode == AutoReplyMode::Static {
        " selected"
    } else {
        ""
    };
    let mode_ai_sel = if account.auto_reply.mode == AutoReplyMode::Ai {
        " selected"
    } else {
        ""
    };

    let content = format!(
        "<p><a href=\"{base_url}/admin/instagram\">&larr; Back to Instagram Accounts</a></p>
        <h1>Edit Instagram Account</h1>
        <p class=\"muted\" style=\"margin-bottom: 1rem;\">@{username}</p>
        <div id=\"toast\"></div>
        <div class=\"card\" style=\"padding:22px\">
            <form hx-put=\"{base_url}/admin/instagram/{id}\" hx-target=\"{hash}toast\" hx-swap=\"innerHTML\">
                <div class=\"form-group\">
                    <label><input type=\"checkbox\" name=\"enabled\" value=\"true\"{enabled_checked}> Account Enabled</label>
                </div>
                <h3 style=\"margin: 1rem 0 0.5rem;\">Auto-Reply</h3>
                <div class=\"form-group\">
                    <label><input type=\"checkbox\" name=\"auto_reply_enabled\" value=\"true\"{ar_enabled_checked}> Enabled</label>
                </div>
                <div class=\"form-group\">
                    <label>Mode</label>
                    <select name=\"auto_reply_mode\">
                        <option value=\"Static\"{mode_static_sel}>Static</option>
                        <option value=\"Ai\"{mode_ai_sel}>AI</option>
                    </select>
                </div>
                <div class=\"form-group\">
                    <label>Prompt / Message</label>
                    <textarea name=\"auto_reply_prompt\" rows=\"3\">{prompt}</textarea>
                </div>
                <div style=\"display: flex; justify-content: flex-end;\">
                    <button type=\"submit\" class=\"btn\">Save</button>
                </div>
            </form>
        </div>",
        base_url = base_url,
        id = html_escape(&account.id),
        username = html_escape(&account.instagram_username),
        enabled_checked = enabled_checked,
        ar_enabled_checked = ar_enabled_checked,
        mode_static_sel = mode_static_sel,
        mode_ai_sel = mode_ai_sel,
        prompt = html_escape(&account.auto_reply.prompt),
        hash = HASH,
    );

    base_html("Edit Instagram Account - Concierge", &content)
}

pub fn admin_lead_forms_list_html(forms: &[LeadCaptureForm], base_url: &str) -> String {
    let rows: String = forms
        .iter()
        .map(|f| {
            let status = if f.enabled { "Active" } else { "Disabled" };
            format!(
                "<tr>
                    <td><a href=\"{base_url}/admin/lead-forms/{id}\">{name}</a></td>
                    <td><code>{slug}</code></td>
                    <td>{status}</td>
                    <td>
                        <a href=\"{base_url}/admin/lead-forms/{id}\" class=\"btn sm\">Edit</a>
                        <button class=\"btn sm\" style=\"border-color:var(--warn);color:var(--warn)\"
                                hx-delete=\"{base_url}/admin/lead-forms/{id}\"
                                hx-confirm=\"Delete this lead form?\"
                                hx-target=\"closest tr\" hx-swap=\"outerHTML\">Delete</button>
                    </td>
                </tr>",
                base_url = base_url,
                id = html_escape(&f.id),
                name = html_escape(&f.name),
                slug = html_escape(&f.slug),
                status = status,
            )
        })
        .collect();

    let empty = if forms.is_empty() {
        "<tr><td colspan=\"4\" class=\"muted\">No lead forms created.</td></tr>"
    } else {
        ""
    };

    let content = format!(
        "<p><a href=\"{base_url}/admin\">&larr; Back to Dashboard</a></p>
        <div style=\"display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;\">
            <h1>Lead Forms</h1>
            <a href=\"{base_url}/admin/lead-forms/new\" class=\"btn\">+ New Form</a>
        </div>
        <div id=\"toast\"></div>
        <div class=\"card\" style=\"padding:22px\">
            <div class=\"table-wrap\"><table>
                <thead><tr><th scope=\"col\">Name</th><th scope=\"col\">Slug</th><th scope=\"col\">Status</th><th></th></tr></thead>
                <tbody>{rows}{empty}</tbody>
            </table></div>
        </div>",
        base_url = base_url,
        rows = rows,
        empty = empty,
    );

    base_html("Lead Forms - Concierge", &content)
}

pub fn admin_lead_form_edit_html(
    form: &LeadCaptureForm,
    whatsapp_accounts: &[WhatsAppAccount],
    base_url: &str,
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

    let mode_static_sel = if form.reply_mode == AutoReplyMode::Static {
        " selected"
    } else {
        ""
    };
    let mode_ai_sel = if form.reply_mode == AutoReplyMode::Ai {
        " selected"
    } else {
        ""
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
        "<p><a href=\"{base_url}/admin/lead-forms\">&larr; Back to Lead Forms</a></p>
        <h1>Edit Lead Form</h1>
        <div id=\"toast\"></div>
        <div class=\"card\" style=\"padding:22px\">
            <form hx-put=\"{base_url}/admin/lead-forms/{id}\" hx-target=\"{hash}toast\" hx-swap=\"innerHTML\">
                <div class=\"form-group\">
                    <label>Name</label>
                    <input type=\"text\" name=\"name\" value=\"{name}\" required>
                </div>
                <div class=\"form-group\">
                    <label><input type=\"checkbox\" name=\"enabled\" value=\"true\"{enabled_checked}> Enabled</label>
                </div>
                <div class=\"form-group\">
                    <label>WhatsApp Account</label>
                    <select name=\"whatsapp_account_id\" required>{wa_options}</select>
                </div>
                <div class=\"form-group\">
                    <label>Reply Mode</label>
                    <select name=\"reply_mode\">
                        <option value=\"Static\"{mode_static_sel}>Static</option>
                        <option value=\"Ai\"{mode_ai_sel}>AI</option>
                    </select>
                </div>
                <div class=\"form-group\">
                    <label>Reply Prompt</label>
                    <textarea name=\"reply_prompt\" rows=\"3\">{reply_prompt}</textarea>
                </div>
                <h3 style=\"margin: 1rem 0 0.5rem;\">Style</h3>
                <div class=\"form-group\">
                    <label>Primary Color</label>
                    <input type=\"color\" name=\"style_primary_color\" value=\"{s_primary}\">
                </div>
                <div class=\"form-group\">
                    <label>Text Color</label>
                    <input type=\"color\" name=\"style_text_color\" value=\"{s_text}\">
                </div>
                <div class=\"form-group\">
                    <label>Background Color</label>
                    <input type=\"color\" name=\"style_background_color\" value=\"{s_bg}\">
                </div>
                <div class=\"form-group\">
                    <label>Border Radius</label>
                    <input type=\"text\" name=\"style_border_radius\" value=\"{s_radius}\">
                </div>
                <div class=\"form-group\">
                    <label>Button Text</label>
                    <input type=\"text\" name=\"style_button_text\" value=\"{s_button}\">
                </div>
                <div class=\"form-group\">
                    <label>Placeholder Text</label>
                    <input type=\"text\" name=\"style_placeholder_text\" value=\"{s_placeholder}\">
                </div>
                <div class=\"form-group\">
                    <label>Success Message</label>
                    <input type=\"text\" name=\"style_success_message\" value=\"{s_success}\">
                </div>
                <div class=\"form-group\">
                    <label>Custom CSS</label>
                    <textarea name=\"style_custom_css\" rows=\"3\">{s_css}</textarea>
                </div>
                <h3 style=\"margin: 1rem 0 0.5rem;\">Allowed Origins</h3>
                <div class=\"form-group\">
                    <textarea name=\"allowed_origins\" rows=\"3\" placeholder=\"https://example.com (one per line)\">{origins}</textarea>
                    <small class=\"muted\">One origin per line. Leave empty to allow all.</small>
                </div>
                <div style=\"display: flex; justify-content: flex-end;\">
                    <button type=\"submit\" class=\"btn\">Save</button>
                </div>
            </form>
        </div>

        <div class=\"card\" style=\"padding:22px\">
            <h3>Embed Code</h3>
            <p class=\"muted\" style=\"margin-bottom: 0.5rem;\">Copy and paste this into your website:</p>
            <div style=\"display:flex;gap:8px;align-items:center\">
                <code style=\"display: block; padding: 0.5rem; flex: 1; overflow-x: auto; white-space: nowrap;\">{embed_code}</code>
                <button class=\"btn sm\" onclick=\"copyUrl(this, '{embed_raw}')\">Copy</button>
            </div>
        </div>",
        base_url = base_url,
        id = html_escape(&form.id),
        name = html_escape(&form.name),
        enabled_checked = enabled_checked,
        wa_options = wa_options,
        mode_static_sel = mode_static_sel,
        mode_ai_sel = mode_ai_sel,
        reply_prompt = html_escape(&form.reply_prompt),
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
    );

    base_html("Edit Lead Form - Concierge", &content)
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
