//! Admin dashboard templates for messaging-first product

use crate::helpers::html_escape;
use crate::types::*;

use super::base::{base_html, BaseStyle};
use super::HASH;

pub fn auth_login_html(base_url: &str, client_id: &str) -> String {
    let redirect_uri = format!("{}/auth/callback", base_url);
    let google_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=online&prompt=select_account",
        urlencoding::encode(client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("openid email profile"),
    );

    let style = BaseStyle::default();
    let content = format!(
        "<div style=\"max-width: 400px; margin: 4rem auto; text-align: center;\">
            <h1 style=\"margin-bottom: 0.5rem;\">Concierge</h1>
            <p style=\"color: {hash}666; margin-bottom: 2rem;\">Sign in to manage your messaging channels.</p>
            <a href=\"{google_url}\" class=\"btn\" style=\"display: inline-flex; align-items: center; gap: 0.5rem; padding: 0.75rem 1.5rem; font-size: 1rem;\">
                <svg width=\"18\" height=\"18\" viewBox=\"0 0 18 18\" xmlns=\"http://www.w3.org/2000/svg\"><path d=\"M17.64 9.2c0-.637-.057-1.251-.164-1.84H9v3.481h4.844a4.14 4.14 0 01-1.796 2.716v2.259h2.908c1.702-1.567 2.684-3.875 2.684-6.615z\" fill=\"#4285F4\"/><path d=\"M9 18c2.43 0 4.467-.806 5.956-2.18l-2.908-2.259c-.806.54-1.837.86-3.048.86-2.344 0-4.328-1.584-5.036-3.711H.957v2.332A8.997 8.997 0 009 18z\" fill=\"#34A853\"/><path d=\"M3.964 10.71A5.41 5.41 0 013.682 9c0-.593.102-1.17.282-1.71V4.958H.957A8.996 8.996 0 000 9c0 1.452.348 2.827.957 4.042l3.007-2.332z\" fill=\"#FBBC05\"/><path d=\"M9 3.58c1.321 0 2.508.454 3.44 1.345l2.582-2.58C13.463.891 11.426 0 9 0A8.997 8.997 0 00.957 4.958L3.964 7.29C4.672 5.163 6.656 3.58 9 3.58z\" fill=\"#EA4335\"/></svg>
                Sign in with Google
            </a>
        </div>",
        google_url = html_escape(&google_url),
        hash = HASH,
    );

    base_html("Sign In - Concierge", &content, &style)
}

pub fn admin_settings_html(base_url: &str) -> String {
    let style = BaseStyle::default();
    let content = format!(
        "<p><a href=\"{base_url}/admin\">&larr; Back to Dashboard</a></p>
        <h1>Settings</h1>
        <div class=\"card\">
            <h2>Account</h2>
            <p style=\"margin-bottom: 1rem;\" class=\"text-muted\">Manage your account.</p>
            <a href=\"{base_url}/auth/logout\" class=\"btn btn-secondary\">Sign Out</a>
        </div>
        <div class=\"card\" style=\"border-color: var(--error-text);\">
            <h2 style=\"color: var(--error-text);\">Delete Account</h2>
            <p style=\"margin-bottom: 1rem;\" class=\"text-muted\">Permanently delete your account and all associated data (WhatsApp accounts, Instagram accounts, lead forms, and message logs). This cannot be undone.</p>
            <button class=\"btn btn-danger\"
                    hx-delete=\"{base_url}/admin/delete-account\"
                    hx-confirm=\"Are you sure? This will permanently delete your account and ALL data. This cannot be undone.\"
                    >Delete My Account</button>
        </div>",
        base_url = base_url,
    );

    base_html("Settings - Concierge", &content, &style)
}

pub fn admin_dashboard_html(
    whatsapp_accounts: &[WhatsAppAccount],
    instagram_accounts: &[InstagramAccount],
    lead_forms: &[LeadCaptureForm],
    base_url: &str,
) -> String {
    let wa_rows: String = whatsapp_accounts
        .iter()
        .map(|a| {
            let enabled = if a.auto_reply.enabled { "Yes" } else { "No" };
            format!(
                "<tr>
                    <td><a href=\"{base_url}/admin/whatsapp/{id}\">{name}</a></td>
                    <td><code>{phone}</code></td>
                    <td>{enabled}</td>
                    <td><a href=\"{base_url}/admin/whatsapp/{id}\" class=\"btn btn-sm\">Edit</a></td>
                </tr>",
                base_url = base_url,
                id = html_escape(&a.id),
                name = html_escape(&a.name),
                phone = html_escape(&a.phone_number),
                enabled = enabled,
            )
        })
        .collect();

    let wa_empty = if whatsapp_accounts.is_empty() {
        "<tr><td colspan=\"4\" class=\"text-muted\">No WhatsApp accounts configured.</td></tr>"
    } else {
        ""
    };

    let ig_rows: String = instagram_accounts
        .iter()
        .map(|a| {
            let enabled = if a.enabled { "Active" } else { "Disabled" };
            format!(
                "<tr>
                    <td><a href=\"{base_url}/admin/instagram/{id}\">@{username}</a></td>
                    <td>{enabled}</td>
                    <td><a href=\"{base_url}/admin/instagram/{id}\" class=\"btn btn-sm\">Edit</a></td>
                </tr>",
                base_url = base_url,
                id = html_escape(&a.id),
                username = html_escape(&a.instagram_username),
                enabled = enabled,
            )
        })
        .collect();

    let ig_empty = if instagram_accounts.is_empty() {
        "<tr><td colspan=\"3\" class=\"text-muted\">No Instagram accounts connected.</td></tr>"
    } else {
        ""
    };

    let lf_rows: String = lead_forms
        .iter()
        .map(|f| {
            let enabled = if f.enabled { "Active" } else { "Disabled" };
            format!(
                "<tr>
                    <td><a href=\"{base_url}/admin/lead-forms/{id}\">{name}</a></td>
                    <td><code>{slug}</code></td>
                    <td>{enabled}</td>
                    <td><a href=\"{base_url}/admin/lead-forms/{id}\" class=\"btn btn-sm\">Edit</a></td>
                </tr>",
                base_url = base_url,
                id = html_escape(&f.id),
                name = html_escape(&f.name),
                slug = html_escape(&f.slug),
                enabled = enabled,
            )
        })
        .collect();

    let lf_empty = if lead_forms.is_empty() {
        "<tr><td colspan=\"4\" class=\"text-muted\">No lead forms created.</td></tr>"
    } else {
        ""
    };

    let style = BaseStyle::default();
    let content = format!(
        "<div style=\"display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;\">
            <h1>Dashboard</h1>
            <a href=\"{base_url}/admin/settings\" class=\"btn btn-secondary\">Settings</a>
        </div>
        <div id=\"toast\"></div>

        <div class=\"card\">
            <div style=\"display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem;\">
                <h2>WhatsApp Accounts</h2>
                <a href=\"{base_url}/admin/whatsapp\" class=\"btn btn-sm\">Manage</a>
            </div>
            <table>
                <thead><tr><th>Name</th><th>Phone</th><th>Auto-Reply</th><th></th></tr></thead>
                <tbody>{wa_rows}{wa_empty}</tbody>
            </table>
        </div>

        <div class=\"card\">
            <div style=\"display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem;\">
                <h2>Instagram Accounts</h2>
                <a href=\"{base_url}/admin/instagram\" class=\"btn btn-sm\">Manage</a>
            </div>
            <table>
                <thead><tr><th>Username</th><th>Status</th><th></th></tr></thead>
                <tbody>{ig_rows}{ig_empty}</tbody>
            </table>
        </div>

        <div class=\"card\">
            <div style=\"display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem;\">
                <h2>Lead Forms</h2>
                <a href=\"{base_url}/admin/lead-forms\" class=\"btn btn-sm\">Manage</a>
            </div>
            <table>
                <thead><tr><th>Name</th><th>Slug</th><th>Status</th><th></th></tr></thead>
                <tbody>{lf_rows}{lf_empty}</tbody>
            </table>
        </div>",
        base_url = base_url,
        wa_rows = wa_rows,
        wa_empty = wa_empty,
        ig_rows = ig_rows,
        ig_empty = ig_empty,
        lf_rows = lf_rows,
        lf_empty = lf_empty,
    );

    base_html("Dashboard - Concierge", &content, &style)
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
                        <a href=\"{base_url}/admin/whatsapp/{id}\" class=\"btn btn-sm\">Edit</a>
                        <button class=\"btn btn-sm btn-danger\"
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
        "<tr><td colspan=\"4\" class=\"text-muted\">No WhatsApp accounts configured.</td></tr>"
    } else {
        ""
    };

    let style = BaseStyle::default();
    let content = format!(
        "<p><a href=\"{base_url}/admin\">&larr; Back to Dashboard</a></p>
        <div style=\"display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;\">
            <h1>WhatsApp Accounts</h1>
            <a href=\"{base_url}/admin/whatsapp/new\" class=\"btn\">+ Connect WhatsApp Number</a>
        </div>
        <div id=\"toast\"></div>
        <div class=\"card\">
            <table>
                <thead><tr><th>Name</th><th>Phone</th><th>Auto-Reply</th><th></th></tr></thead>
                <tbody>{rows}{empty}</tbody>
            </table>
        </div>",
        base_url = base_url,
        rows = rows,
        empty = empty,
    );

    base_html("WhatsApp Accounts - Concierge", &content, &style)
}

pub fn admin_whatsapp_signup_html(
    base_url: &str,
    app_id: &str,
    config_id: &str,
    state: &str,
) -> String {
    let style = BaseStyle::default();
    let content = format!(
        r#"<p><a href="{base_url}/admin/whatsapp">&larr; Back to WhatsApp Accounts</a></p>
        <h1>Connect WhatsApp Number</h1>
        <div class="card" style="text-align: center; padding: 2rem;">
            <p class="text-muted" style="margin-bottom: 1.5rem;">Click the button below to register your phone number through Meta's WhatsApp setup flow.</p>
            <div id="signup-error" style="color: var(--error-text); margin-bottom: 1rem;"></div>
            <button id="signup-btn" class="btn" style="padding: 0.75rem 2rem; font-size: 1rem;" onclick="launchSignup()">
                Connect WhatsApp Number
            </button>
            <p class="text-muted" style="margin-top: 1.5rem; font-size: 0.85rem;">
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
                    version: 'v21.0'
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
    );

    base_html("Connect WhatsApp - Concierge", &content, &style)
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

    let style = BaseStyle::default();
    let content = format!(
        "<p><a href=\"{base_url}/admin/whatsapp\">&larr; Back to WhatsApp Accounts</a></p>
        <h1>Edit WhatsApp Account</h1>
        <div id=\"toast\"></div>
        <div class=\"card\">
            <form hx-put=\"{base_url}/admin/whatsapp/{id}\" hx-target=\"{hash}toast\" hx-swap=\"innerHTML\">
                <div class=\"form-group\">
                    <label>Name</label>
                    <input type=\"text\" name=\"name\" value=\"{name}\" required>
                </div>
                <div class=\"form-group\">
                    <label>Phone Number</label>
                    <input type=\"text\" name=\"phone_number\" value=\"{phone}\" placeholder=\"+1234567890\" required>
                </div>
                <div class=\"form-group\">
                    <label>Phone Number ID</label>
                    <input type=\"text\" name=\"phone_number_id\" value=\"{phone_id}\" placeholder=\"Meta phone number ID\" required style=\"font-family: monospace;\">
                </div>
                <h3 style=\"margin: 1rem 0 0.5rem;\">Auto-Reply</h3>
                <div class=\"form-group\">
                    <label><input type=\"checkbox\" name=\"auto_reply_enabled\" value=\"true\"{enabled_checked}> Enabled</label>
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

    base_html("Edit WhatsApp Account - Concierge", &content, &style)
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
                        <a href=\"{base_url}/admin/instagram/{id}\" class=\"btn btn-sm\">Edit</a>
                        <button class=\"btn btn-sm btn-danger\"
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
        "<tr><td colspan=\"4\" class=\"text-muted\">No Instagram accounts connected.</td></tr>"
    } else {
        ""
    };

    let style = BaseStyle::default();
    let content = format!(
        "<p><a href=\"{base_url}/admin\">&larr; Back to Dashboard</a></p>
        <div style=\"display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;\">
            <h1>Instagram Accounts</h1>
            <a href=\"{base_url}/instagram/auth/{tenant_id}\" class=\"btn\">+ Connect Account</a>
        </div>
        <div id=\"toast\"></div>
        <div class=\"card\">
            <table>
                <thead><tr><th>Username</th><th>Status</th><th>Auto-Reply</th><th></th></tr></thead>
                <tbody>{rows}{empty}</tbody>
            </table>
        </div>",
        base_url = base_url,
        tenant_id = html_escape(tenant_id),
        rows = rows,
        empty = empty,
    );

    base_html("Instagram Accounts - Concierge", &content, &style)
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

    let style = BaseStyle::default();
    let content = format!(
        "<p><a href=\"{base_url}/admin/instagram\">&larr; Back to Instagram Accounts</a></p>
        <h1>Edit Instagram Account</h1>
        <p class=\"text-muted\" style=\"margin-bottom: 1rem;\">@{username}</p>
        <div id=\"toast\"></div>
        <div class=\"card\">
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

    base_html("Edit Instagram Account - Concierge", &content, &style)
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
                        <a href=\"{base_url}/admin/lead-forms/{id}\" class=\"btn btn-sm\">Edit</a>
                        <button class=\"btn btn-sm btn-danger\"
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
        "<tr><td colspan=\"4\" class=\"text-muted\">No lead forms created.</td></tr>"
    } else {
        ""
    };

    let style = BaseStyle::default();
    let content = format!(
        "<p><a href=\"{base_url}/admin\">&larr; Back to Dashboard</a></p>
        <div style=\"display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;\">
            <h1>Lead Forms</h1>
            <a href=\"{base_url}/admin/lead-forms/new\" class=\"btn\">+ New Form</a>
        </div>
        <div id=\"toast\"></div>
        <div class=\"card\">
            <table>
                <thead><tr><th>Name</th><th>Slug</th><th>Status</th><th></th></tr></thead>
                <tbody>{rows}{empty}</tbody>
            </table>
        </div>",
        base_url = base_url,
        rows = rows,
        empty = empty,
    );

    base_html("Lead Forms - Concierge", &content, &style)
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

    let style = BaseStyle::default();
    let content = format!(
        "<p><a href=\"{base_url}/admin/lead-forms\">&larr; Back to Lead Forms</a></p>
        <h1>Edit Lead Form</h1>
        <div id=\"toast\"></div>
        <div class=\"card\">
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
                    <input type=\"text\" name=\"style_primary_color\" value=\"{s_primary}\">
                </div>
                <div class=\"form-group\">
                    <label>Text Color</label>
                    <input type=\"text\" name=\"style_text_color\" value=\"{s_text}\">
                </div>
                <div class=\"form-group\">
                    <label>Background Color</label>
                    <input type=\"text\" name=\"style_background_color\" value=\"{s_bg}\">
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
                    <small class=\"text-muted\">One origin per line. Leave empty to allow all.</small>
                </div>
                <div style=\"display: flex; justify-content: flex-end;\">
                    <button type=\"submit\" class=\"btn\">Save</button>
                </div>
            </form>
        </div>

        <div class=\"card\">
            <h3>Embed Code</h3>
            <p class=\"text-muted\" style=\"margin-bottom: 0.5rem;\">Copy and paste this into your website:</p>
            <div class=\"url-cell\">
                <code style=\"display: block; padding: 0.5rem; flex: 1; overflow-x: auto; white-space: nowrap;\">{embed_code}</code>
                <button class=\"btn btn-copy btn-sm\" onclick=\"copyUrl(this, '{embed_raw}')\">Copy</button>
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

    base_html("Edit Lead Form - Concierge", &content, &style)
}

pub fn admin_success_html(message: &str) -> String {
    format!("<div class=\"success\">{}</div>", html_escape(message))
}

#[allow(dead_code)]
pub fn admin_error_html(message: &str) -> String {
    format!("<div class=\"error\">{}</div>", html_escape(message))
}
