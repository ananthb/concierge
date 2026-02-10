//! Form editor, responses, and public form rendering templates

use crate::helpers::*;
use crate::types::*;

use super::base::AvailableChannels;

pub fn form_editor_html(form: Option<&FormConfig>, admin_email: &str, channels: &AvailableChannels) -> String {
    let is_new = form.is_none();
    let form = form.cloned().unwrap_or_else(|| FormConfig {
        slug: generate_slug(),
        name: "New Form".into(),
        title: "Contact Us".into(),
        submit_button_text: "Send Message".into(),
        success_message: "Thank you! Your message has been received.".into(),
        allowed_origins: vec![],
        fields: FormConfig::default_fields(),
        style: FormStyle::default(),
        responders: vec![],
        digest: DigestConfig::default(),
        google_sheet_url: None,
        instagram_sources: vec![],
        archived: false,
        created_at: now_iso(),
        updated_at: now_iso(),
    });

    let fields_json = serde_json::to_string(&form.fields).unwrap_or_else(|_| "[]".into());
    let style_json = serde_json::to_string(&form.style).unwrap_or_else(|_| "{}".into());
    let responders_json = serde_json::to_string(&form.responders).unwrap_or_else(|_| "[]".into());
    let digest_json = serde_json::to_string(&form.digest)
        .unwrap_or_else(|_| r#"{"frequency":"none","recipients":[]}"#.into());
    let google_sheet_url = form.google_sheet_url.as_deref().unwrap_or("");

    // Build channel options based on availability
    let mut channel_options = Vec::new();
    if channels.twilio_sms {
        channel_options.push(("twilio_sms", "Twilio SMS"));
    }
    if channels.twilio_whatsapp {
        channel_options.push(("twilio_whatsapp", "Twilio WhatsApp"));
    }
    if channels.twilio_email {
        channel_options.push(("twilio_email", "Twilio Email"));
    }
    if channels.resend_email {
        channel_options.push(("resend_email", "Resend Email"));
    }
    let has_channels = !channel_options.is_empty();
    let default_channel = channel_options.first().map(|(c, _)| *c).unwrap_or("resend_email");

    // Build JS channel options
    let js_channel_options: String = channel_options
        .iter()
        .map(|(value, label)| format!("<option value=\"{}\" ${{r.channel==='{}'?'selected':''}}>{}</option>", value, value, label))
        .collect::<Vec<_>>()
        .join("\\n                            ");

    let js_escape = |s: &str| {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    };
    let admin_email_escaped = js_escape(admin_email);
    let slug_escaped = js_escape(&form.slug);
    let is_new_str = if is_new { "true" } else { "false" };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} - Form Editor</title>
    <link rel="icon" type="image/svg+xml" href="/logo.svg">
    <style>
        * {{ box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 0; padding: 1rem; background: #f5f5f5; line-height: 1.5; }}
        .container {{ max-width: 1200px; margin: 0 auto; }}
        h1, h2, h3 {{ color: #333; }}
        a {{ color: #0070f3; }}
        .card {{ background: white; padding: 1.5rem; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); margin-bottom: 1.5rem; }}
        .form-group {{ margin-bottom: 1rem; }}
        .form-group label {{ display: block; margin-bottom: 0.25rem; font-weight: 500; }}
        .form-group input, .form-group textarea, .form-group select {{ width: 100%; padding: 0.5rem; border: 1px solid #ddd; border-radius: 4px; font-size: 1rem; }}
        .form-group input:focus, .form-group textarea:focus, .form-group select:focus {{ outline: none; border-color: #0070f3; }}
        .btn {{ display: inline-block; padding: 0.5rem 1rem; background: #0070f3; color: white; text-decoration: none; border: none; border-radius: 4px; cursor: pointer; font-size: 0.875rem; font-family: inherit; line-height: 1.2; vertical-align: middle; }}
        .btn:hover {{ opacity: 0.9; }}
        .btn-secondary {{ background: #6c757d; }}
        .btn-danger {{ background: #dc3545; }}
        .btn-sm {{ padding: 0.25rem 0.5rem; font-size: 0.75rem; }}
        .field-item {{ background: #f8f9fa; padding: 1rem; border-radius: 4px; margin-bottom: 0.5rem; display: flex; gap: 1rem; align-items: center; flex-wrap: wrap; }}
        .field-item > input[type="text"] {{ flex: 1; min-width: 120px; }}
        .field-item > select {{ flex: 0 0 auto; min-width: 100px; }}
        .field-item > label {{ flex: 0 0 auto; white-space: nowrap; display: flex; align-items: center; gap: 0.25rem; }}
        .field-item .actions {{ flex: 0 0 auto; display: flex; gap: 0.5rem; }}
        .style-grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 1rem; }}
        .color-input {{ display: flex; gap: 0.5rem; align-items: center; }}
        .color-input input[type="color"] {{ width: 50px; height: 38px; padding: 2px; }}
        .color-input input[type="text"] {{ flex: 1; }}
        code {{ background: #e9ecef; padding: 0.2rem 0.4rem; border-radius: 3px; }}
        small {{ color: #666; }}
        .tabs {{ display: flex; gap: 0.5rem; margin-bottom: 1.5rem; flex-wrap: wrap; }}
        .tab {{ padding: 0.5rem 1rem; background: #e9ecef; border-radius: 4px; cursor: pointer; border: none; font-size: 1rem; font-family: inherit; }}
        .tab.active {{ background: #0070f3; color: white; }}
        .tab-content {{ display: none; }}
        .tab-content.active {{ display: block; }}
        .preview-frame {{ border: 1px solid #ddd; border-radius: 4px; min-height: 400px; background: white; }}
        table {{ width: 100%; border-collapse: collapse; }}
        th, td {{ padding: 0.5rem; text-align: left; border-bottom: 1px solid #eee; }}
        th {{ background: #f8f9fa; font-weight: 600; }}
    </style>
</head>
<body>
    <div class="container">
        <p><a href="/admin">&larr; Back to Dashboard</a></p>
        <h1>{header}</h1>
        {archived_notice}

        <div class="tabs">
            <button class="tab active" onclick="showTab('basic')">Basic Settings</button>
            <button class="tab" onclick="showTab('fields')">Fields</button>
            {responders_tab}
            {digest_tab}
            <button class="tab" onclick="showTab('style')">Styling</button>
            <button class="tab" onclick="showTab('preview')">Preview</button>
        </div>

        <form id="formEditor" onsubmit="saveForm(event); return false;">
            <div id="tab-basic" class="tab-content active">
                <div class="card">
                    <h3>Basic Settings</h3>
                    <div class="form-group">
                        <label>Form Name (internal)</label>
                        <input type="text" name="name" value="{name}" required oninput="autoSlug(this.value)">
                    </div>
                    <div class="form-group">
                        <label>Slug (URL path)</label>
                        <input type="text" name="slug" value="{slug}" required pattern="[a-z0-9_]+(-[a-z0-9_]+)*" title="Lowercase letters, numbers, hyphens, underscores only"
                            oninput="document.getElementById('slug-preview').textContent=this.value">
                        <small style="color:#666;">Form will be at: <code>/f/<span id="slug-preview">{slug}</span></code></small>
                    </div>
                    <div class="form-group">
                        <label>Form Title (displayed to users)</label>
                        <input type="text" name="title" value="{form_title}" required>
                    </div>
                    <div class="form-group">
                        <label>Submit Button Text</label>
                        <input type="text" name="submit_button_text" value="{submit_button_text}" required>
                    </div>
                    <div class="form-group">
                        <label>Success Message</label>
                        <textarea name="success_message" rows="2">{success_message}</textarea>
                    </div>
                    <div class="form-group">
                        <label>Allowed Origins (one per line)</label>
                        <textarea name="allowed_origins" rows="3" placeholder="https://example.com&#10;https://www.example.com">{allowed_origins}</textarea>
                        <small style="color:#666;">Leave empty to allow all origins (not recommended for production)</small>
                    </div>
                    <div class="form-group">
                        <label>Google Sheet URL (optional)</label>
                        <input type="text" name="google_sheet_url" value="{google_sheet_url}" placeholder="https://docs.google.com/spreadsheets/d/...">
                        <small style="color:#666;">Submissions will be appended as rows. Share the sheet with your service account email.</small>
                    </div>
                </div>
            </div>

            <div id="tab-fields" class="tab-content">
                <div class="card">
                    <h3>Form Fields</h3>
                    <div id="fields-list"></div>
                    <button type="button" class="btn btn-secondary" onclick="addField()">+ Add Field</button>
                </div>
            </div>

            {responders_content}

            {digest_content}

            <div id="tab-style" class="tab-content">
                <div class="card">
                    <h3>Display Options</h3>
                    <div class="form-group">
                        <label style="display:flex;align-items:center;gap:0.5rem;cursor:pointer;">
                            <input type="checkbox" id="show-title" onchange="style.show_title=this.checked" style="width:auto;">
                            Show form title
                        </label>
                        <small style="color:#666;">Uncheck to hide the title when embedding the form</small>
                    </div>
                </div>
                <div class="card">
                    <h3>CSS Variables</h3>
                    <p style="color:#666;margin-bottom:1rem;">Customize the form appearance using CSS variables.</p>
                    <div class="style-grid" id="style-inputs"></div>
                </div>
                <div class="card">
                    <h3>Custom CSS</h3>
                    <p style="color:#666;margin-bottom:0.5rem;">Add custom CSS rules. Available classes: <code>.contact-form</code>, <code>.form-group</code>, <code>label</code>, <code>input</code>, <code>textarea</code>, <code>button</code>, <code>.success</code>, <code>.error</code></p>
                    <div class="form-group" style="margin-bottom:0;">
                        <textarea id="custom-css" rows="8" style="font-family:monospace;font-size:0.9rem;" placeholder="/* Example */
.contact-form {{ max-width: 500px; }}
button {{ text-transform: uppercase; }}" onchange="style.custom_css=this.value"></textarea>
                    </div>
                </div>
            </div>

            <div id="tab-preview" class="tab-content">
                <div class="card">
                    <h3>Preview</h3>
                    <iframe id="preview-frame" class="preview-frame" style="width:100%;height:500px;border:none;"></iframe>
                </div>
            </div>

            {danger_zone}

            <div class="card" style="display:flex;gap:1rem;justify-content:flex-end;">
                <a href="/admin" class="btn btn-secondary">Cancel</a>
                <button type="submit" id="save-btn" class="btn">{save_button}</button>
            </div>
        </form>
    </div>

    <script>
        let fields = {fields_json};
        let style = {style_json};
        if (style.show_title === undefined) style.show_title = true;
        let responders = {responders_json};
        let digest = {digest_json};
        const isNew = {is_new};
        const originalSlug = "{slug_js}";
        const adminEmail = "{admin_email_js}";

        const styleLabels = {{
            font_family: 'Font Family',
            font_size: 'Font Size',
            text_color: 'Text Color',
            text_muted: 'Muted Text',
            bg_color: 'Background',
            form_bg: 'Form Background',
            border_color: 'Border Color',
            border_radius: 'Border Radius',
            primary_color: 'Primary Color',
            primary_hover: 'Primary Hover',
            input_padding: 'Input Padding',
            success_bg: 'Success Background',
            success_color: 'Success Text',
            success_border: 'Success Border',
            error_bg: 'Error Background',
            error_color: 'Error Text',
            error_border: 'Error Border'
        }};

        function autoSlug(name) {{
            if (!isNew) return;
            const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
            document.querySelector('input[name="slug"]').value = slug;
            document.getElementById('slug-preview').textContent = slug;
        }}

        function showTab(name) {{
            document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(t => t.classList.remove('active'));
            document.querySelector(`.tab-content#tab-${{name}}`).classList.add('active');
            event.target.classList.add('active');
            if (name === 'preview') updatePreview();
        }}

        function renderFields() {{
            const list = document.getElementById('fields-list');
            list.innerHTML = fields.map((f, i) => `
                <div class="field-item">
                    <input type="text" value="${{f.label}}" onchange="fields[${{i}}].label=this.value" placeholder="Label">
                    <input type="text" value="${{f.id}}" onchange="fields[${{i}}].id=this.value" placeholder="Field ID" style="max-width:120px;">
                    <select onchange="fields[${{i}}].field_type=this.value">
                        <option value="text" ${{f.field_type==='text'?'selected':''}}>Text</option>
                        <option value="email" ${{f.field_type==='email'?'selected':''}}>Email</option>
                        <option value="mobile" ${{f.field_type==='mobile'?'selected':''}}>Mobile</option>
                        <option value="long_text" ${{f.field_type==='long_text'?'selected':''}}>Long Text</option>
                        <option value="file" ${{f.field_type==='file'?'selected':''}}>File Upload</option>
                    </select>
                    <label><input type="checkbox" ${{f.required?'checked':''}} onchange="fields[${{i}}].required=this.checked"> Required</label>
                    <div class="actions">
                        <button type="button" class="btn btn-sm btn-secondary" onclick="moveField(${{i}},-1)" ${{i===0?'disabled':''}}>↑</button>
                        <button type="button" class="btn btn-sm btn-secondary" onclick="moveField(${{i}},1)" ${{i===fields.length-1?'disabled':''}}>↓</button>
                        <button type="button" class="btn btn-sm btn-danger" onclick="removeField(${{i}})">×</button>
                    </div>
                </div>
            `).join('');
        }}

        function addField() {{
            fields.push({{ id: 'field_' + Date.now(), label: 'New Field', field_type: 'text', required: false, placeholder: '' }});
            renderFields();
        }}

        function removeField(i) {{ fields.splice(i, 1); renderFields(); }}
        function moveField(i, dir) {{ [fields[i], fields[i+dir]] = [fields[i+dir], fields[i]]; renderFields(); }}

        function getTargetFields(channel) {{
            const isEmail = channel === 'twilio_email' || channel === 'resend_email';
            if (channel === 'meta_whatsapp') return [];
            return fields.filter(f => isEmail ? f.field_type === 'email' : f.field_type === 'mobile');
        }}

        function renderResponders() {{
            const list = document.getElementById('responders-list');
            if (!list) return;
            list.innerHTML = responders.map((r, i) => {{
                const isEmail = r.channel === 'twilio_email' || r.channel === 'resend_email';
                const isMetaWhatsapp = r.channel === 'meta_whatsapp';
                const targetFields = getTargetFields(r.channel);
                const targetOptions = targetFields.map(f => `<option value="${{f.id}}" ${{r.target_field===f.id?'selected':''}}>${{f.label}}</option>`).join('');
                const useAi = r.use_ai || false;

                return `<div class="card" style="margin-bottom:1rem;padding:1rem;">
                    <div style="display:flex;gap:1rem;margin-bottom:0.5rem;align-items:center;flex-wrap:wrap;">
                        <input type="text" value="${{r.name}}" onchange="responders[${{i}}].name=this.value" placeholder="Responder Name" style="flex:1;min-width:150px;">
                        <select onchange="responders[${{i}}].channel=this.value;renderResponders();">
                            {js_channel_options}
                        </select>
                        ${{!isMetaWhatsapp ? `<select onchange="responders[${{i}}].target_field=this.value" style="flex:0.8;">
                            ${{targetOptions || '<option value="">(No matching fields)</option>'}}
                        </select>` : ''}}
                        <label style="white-space:nowrap;"><input type="checkbox" ${{r.enabled?'checked':''}} onchange="responders[${{i}}].enabled=this.checked"> Enabled</label>
                        <button type="button" class="btn btn-sm btn-danger" onclick="removeResponder(${{i}})">Delete</button>
                    </div>
                    ${{isEmail ? `<div class="form-group" style="margin-bottom:0.5rem;">
                        <input type="text" value="${{r.subject}}" onchange="responders[${{i}}].subject=this.value" placeholder="Email Subject">
                    </div>` : ''}}
                    <div class="form-group" style="margin-bottom:0.5rem;">
                        <textarea rows="3" onchange="responders[${{i}}].body=this.value" placeholder="${{useAi ? 'AI system prompt describing how to respond' : 'Message body. Use {{name}} to insert field values.'}}">${{r.body}}</textarea>
                    </div>
                    <label style="display:flex;align-items:center;gap:0.25rem;cursor:pointer;">
                        <input type="checkbox" ${{useAi?'checked':''}} onchange="responders[${{i}}].use_ai=this.checked;renderResponders();">
                        <span>Use AI to generate response</span>
                    </label>
                </div>`;
            }}).join('');

            if (responders.length === 0) {{
                list.innerHTML = '<p style="color:#666;text-align:center;">No responders configured.</p>';
            }}
        }}

        function addResponder() {{
            responders.push({{
                id: 'resp_' + Date.now(),
                name: 'New Responder',
                channel: '{default_channel}',
                target_field: fields.find(f => f.field_type === 'email')?.id || '',
                subject: 'Thank you for your submission',
                body: "Hi {{{{name}}}},\\n\\nThank you for contacting us.",
                enabled: true,
                use_ai: false
            }});
            renderResponders();
        }}

        function removeResponder(i) {{ responders.splice(i, 1); renderResponders(); }}

        function renderDigest() {{
            const freqEl = document.getElementById('digest-frequency');
            const chanEl = document.getElementById('digest-channel');
            const recEl = document.getElementById('digest-recipients');
            if (freqEl) freqEl.value = digest.frequency || 'none';
            if (chanEl) chanEl.value = digest.channel || '';
            if (recEl) recEl.value = (digest.recipients || []).join('\\n');
        }}

        function renderStyleInputs() {{
            const container = document.getElementById('style-inputs');
            container.innerHTML = Object.entries(styleLabels).map(([key, label]) => {{
                const val = style[key] || '';
                const isColor = key.includes('color') || key.includes('bg') || key.includes('border');
                if (isColor && val.startsWith('#')) {{
                    return `<div class="form-group">
                        <label>${{label}}</label>
                        <div class="color-input">
                            <input type="color" value="${{val}}" onchange="style['${{key}}']=this.value;this.nextElementSibling.value=this.value">
                            <input type="text" value="${{val}}" onchange="style['${{key}}']=this.value;this.previousElementSibling.value=this.value">
                        </div>
                    </div>`;
                }}
                return `<div class="form-group"><label>${{label}}</label><input type="text" value="${{val}}" onchange="style['${{key}}']=this.value"></div>`;
            }}).join('');
            document.getElementById('custom-css').value = style.custom_css || '';
            document.getElementById('show-title').checked = style.show_title;
        }}

        function updatePreview() {{
            const form = document.getElementById('formEditor');
            const fd = new FormData(form);
            const previewData = {{ title: fd.get('title'), fields: fields, style: style, submit_button_text: fd.get('submit_button_text') }};
            document.getElementById('preview-frame').srcdoc = generatePreviewHtml(previewData);
        }}

        function generatePreviewHtml(data) {{
            const s = data.style;
            const fieldsHtml = data.fields.map(f => {{
                let input = '';
                switch(f.field_type) {{
                    case 'long_text': input = `<textarea id="${{f.id}}" name="${{f.id}}" ${{f.required?'required':''}}></textarea>`; break;
                    case 'file': input = `<input type="file" id="${{f.id}}" name="${{f.id}}" ${{f.required?'required':''}}>`; break;
                    case 'email': input = `<input type="email" id="${{f.id}}" name="${{f.id}}" ${{f.required?'required':''}}>`; break;
                    case 'mobile': input = `<input type="tel" id="${{f.id}}" name="${{f.id}}" ${{f.required?'required':''}}>`; break;
                    default: input = `<input type="text" id="${{f.id}}" name="${{f.id}}" ${{f.required?'required':''}}>`;
                }}
                return `<div class="form-group"><label for="${{f.id}}">${{f.label}}</label>${{input}}</div>`;
            }}).join('');

            return `<!DOCTYPE html><html><head><style>
*,*::before,*::after{{box-sizing:border-box;margin:0;padding:0;}}
:root{{--cf-font-family:${{s.font_family}};--cf-font-size:${{s.font_size}};--cf-text-color:${{s.text_color}};--cf-bg-color:${{s.bg_color}};--cf-form-bg:${{s.form_bg}};--cf-border-color:${{s.border_color}};--cf-border-radius:${{s.border_radius}};--cf-primary-color:${{s.primary_color}};--cf-primary-hover:${{s.primary_hover}};--cf-input-padding:${{s.input_padding}};}}
body{{font-family:var(--cf-font-family);font-size:var(--cf-font-size);color:var(--cf-text-color);padding:2rem;background:var(--cf-bg-color);}}
h1{{margin-bottom:1.5rem;}}
.contact-form{{background:var(--cf-form-bg);padding:2rem;border-radius:var(--cf-border-radius);}}
.form-group{{margin-bottom:1rem;}}
label{{display:block;margin-bottom:0.5rem;font-weight:500;}}
input,textarea{{width:100%;padding:var(--cf-input-padding);border:1px solid var(--cf-border-color);border-radius:var(--cf-border-radius);}}
textarea{{min-height:120px;resize:vertical;}}
button{{background:var(--cf-primary-color);color:white;padding:var(--cf-input-padding) 1.5rem;border:none;border-radius:var(--cf-border-radius);cursor:pointer;width:100%;}}
button:hover{{background:var(--cf-primary-hover);}}
${{s.custom_css||''}}
</style></head><body>
${{s.show_title?`<h1>${{data.title}}</h1>`:''}}
<form class="contact-form">${{fieldsHtml}}<button type="submit">${{data.submit_button_text}}</button></form>
</body></html>`;
        }}

        async function saveForm(e) {{
            e.preventDefault();
            const btn = document.getElementById('save-btn');
            const originalText = btn.textContent;
            btn.disabled = true;
            btn.textContent = 'Saving...';
            try {{
                const form = e.target;
                const fd = new FormData(form);
                style.show_title = document.getElementById('show-title').checked;
                style.custom_css = document.getElementById('custom-css').value;

                const data = {{
                    slug: fd.get('slug'),
                    name: fd.get('name'),
                    title: fd.get('title'),
                    submit_button_text: fd.get('submit_button_text'),
                    success_message: fd.get('success_message'),
                    allowed_origins: fd.get('allowed_origins').split('\\n').map(s => s.trim()).filter(s => s),
                    google_sheet_url: fd.get('google_sheet_url') || null,
                    fields: fields,
                    style: style,
                    responders: responders,
                    digest: digest
                }};

                const url = isNew ? '/admin/forms' : '/admin/forms/' + originalSlug;
                const method = isNew ? 'POST' : 'PUT';

                const resp = await fetch(url, {{
                    method,
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify(data),
                    credentials: 'same-origin'
                }});

                if (resp.ok) {{
                    btn.textContent = 'Saved!';
                    await new Promise(r => setTimeout(r, 500));
                    window.location.href = '/admin';
                }} else {{
                    btn.disabled = false;
                    btn.textContent = originalText;
                    alert('Error: ' + await resp.text());
                }}
            }} catch (err) {{
                btn.disabled = false;
                btn.textContent = originalText;
                alert('Error: ' + err.message);
            }}
        }}

        async function deleteForm() {{
            if (!confirm('Are you sure you want to permanently delete this form? This action cannot be undone.')) {{
                return;
            }}
            try {{
                const resp = await fetch('/admin/forms/' + originalSlug, {{
                    method: 'DELETE',
                    credentials: 'same-origin'
                }});
                if (resp.ok) {{
                    window.location.href = '/admin';
                }} else {{
                    alert('Error: ' + await resp.text());
                }}
            }} catch (err) {{
                alert('Error: ' + err.message);
            }}
        }}

        renderFields();
        renderStyleInputs();
        renderResponders();
        renderDigest();
    </script>
</body>
</html>"##,
        title = if is_new {
            "New Form".to_string()
        } else if form.archived {
            format!("{} (Archived)", html_escape(&form.name))
        } else {
            html_escape(&form.name)
        },
        header = if is_new {
            "Create New Form"
        } else if form.archived {
            "View Archived Form"
        } else {
            "Edit Form"
        },
        name = html_escape(&form.name),
        slug = html_escape(&form.slug),
        form_title = html_escape(&form.title),
        submit_button_text = html_escape(&form.submit_button_text),
        success_message = html_escape(&form.success_message),
        allowed_origins = form.allowed_origins.join("\n"),
        google_sheet_url = html_escape(google_sheet_url),
        fields_json = fields_json,
        style_json = style_json,
        responders_json = responders_json,
        digest_json = digest_json,
        slug_js = slug_escaped,
        admin_email_js = admin_email_escaped,
        is_new = is_new_str,
        save_button = if is_new {
            "Create Form"
        } else {
            "Save Changes"
        },
        archived_notice = if form.archived {
            r#"<div class="card" style="background: #fff3cd; border-color: #ffc107; margin-bottom: 1rem;">
                <p style="margin: 0; color: #856404;"><strong>This form is archived.</strong> It is read-only. Unarchive from the dashboard to make changes.</p>
            </div>"#
        } else {
            ""
        },
        danger_zone = if !is_new && !form.archived {
            r#"<div class="card" style="border-color: #dc3545;">
                <h3 style="color: #dc3545;">Danger Zone</h3>
                <p style="margin-bottom: 1rem; color: #666;">Permanently delete this form and all its submissions.</p>
                <button type="button" class="btn btn-danger" onclick="deleteForm()">Delete Form</button>
            </div>"#.to_string()
        } else {
            String::new()
        },
        responders_tab = if has_channels {
            r#"<button class="tab" onclick="showTab('responders')">Responders</button>"#
        } else {
            ""
        },
        digest_tab = if channels.twilio_email || channels.resend_email {
            r#"<button class="tab" onclick="showTab('digest')">Digest</button>"#
        } else {
            ""
        },
        responders_content = if has_channels {
            r#"<div id="tab-responders" class="tab-content">
                <div class="card">
                    <h3>Auto-Responders</h3>
                    <p style="color:#666;margin-bottom:1rem;">Send automatic acknowledgement messages when forms are submitted.</p>
                    <div id="responders-list"></div>
                    <button type="button" class="btn btn-secondary" onclick="addResponder()">+ Add Responder</button>
                </div>
            </div>"#.to_string()
        } else {
            String::new()
        },
        digest_content = if channels.twilio_email || channels.resend_email {
            let digest_channel_options = {
                let mut opts = vec![r#"<option value="">Select channel...</option>"#.to_string()];
                if channels.twilio_email {
                    opts.push(r#"<option value="twilio_email">Twilio Email (SendGrid)</option>"#.to_string());
                }
                if channels.resend_email {
                    opts.push(r#"<option value="resend_email">Resend Email</option>"#.to_string());
                }
                opts.join("\n                            ")
            };
            format!(r#"<div id="tab-digest" class="tab-content">
                <div class="card">
                    <h3>Response Digest</h3>
                    <p style="color:#666;margin-bottom:1rem;">Receive periodic email summaries of new form submissions.</p>
                    <div class="form-group">
                        <label>Frequency</label>
                        <select id="digest-frequency" onchange="digest.frequency=this.value">
                            <option value="none">Disabled</option>
                            <option value="daily">Daily</option>
                            <option value="weekly">Weekly</option>
                        </select>
                    </div>
                    <div class="form-group">
                        <label>Email Channel</label>
                        <select id="digest-channel" onchange="digest.channel=this.value||null">
                            {digest_channel_options}
                        </select>
                        <small style="color:#666;">Configure email credentials in wrangler secrets</small>
                    </div>
                    <div class="form-group">
                        <label>Recipients (one email per line)</label>
                        <textarea id="digest-recipients" rows="3" onchange="digest.recipients=this.value.split('\n').map(s=>s.trim()).filter(s=>s)" placeholder="admin@example.com"></textarea>
                        <small style="color:#666;">Leave empty to use your login email ({admin_email})</small>
                    </div>
                </div>
            </div>"#, digest_channel_options = digest_channel_options, admin_email = html_escape(admin_email))
        } else {
            String::new()
        },
        js_channel_options = js_channel_options,
        default_channel = default_channel
    )
}

pub fn responses_view_html(form: &FormConfig, submissions: &[Submission]) -> String {
    let rows: String = if submissions.is_empty() {
        format!(
            r#"<tr><td colspan="{}" style="text-align:center;color:#666;padding:2rem;">No submissions yet.</td></tr>"#,
            form.fields.len() + 2
        )
    } else {
        submissions
            .iter()
            .map(|s| {
                let field_cells: String = form
                    .fields
                    .iter()
                    .map(|f| {
                        let value = s
                            .fields_data
                            .get(&f.id)
                            .map(|v| match v {
                                serde_json::Value::String(s) => html_escape(s),
                                _ => html_escape(&v.to_string()),
                            })
                            .unwrap_or_else(|| "-".to_string());
                        format!("<td>{}</td>", value)
                    })
                    .collect();
                format!(
                    "<tr><td>{}</td>{}<td>{}</td></tr>",
                    s.id,
                    field_cells,
                    html_escape(&s.created_at)
                )
            })
            .collect()
    };

    let header_cells: String = form
        .fields
        .iter()
        .map(|f| format!("<th>{}</th>", html_escape(&f.label)))
        .collect();

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Responses - {form_name}</title>
    <link rel="icon" type="image/svg+xml" href="/logo.svg">
    <style>
        * {{ box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 0; padding: 2rem; background: #f5f5f5; }}
        .container {{ max-width: 1400px; margin: 0 auto; }}
        h1 {{ color: #333; margin: 0; }}
        .btn {{ display: inline-block; padding: 0.5rem 1rem; background: #0070f3; color: white; text-decoration: none; border: none; border-radius: 4px; cursor: pointer; font-size: 0.9rem; }}
        .btn:hover {{ background: #0060df; }}
        .btn-secondary {{ background: #6c757d; }}
        .header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.5rem; }}
        table {{ width: 100%; border-collapse: collapse; background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        th, td {{ padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #eee; }}
        th {{ background: #f8f9fa; font-weight: 600; }}
    </style>
</head>
<body>
    <div class="container">
        <p><a href="/admin">&larr; Back to Dashboard</a></p>
        <div class="header">
            <h1>Responses for {form_name}</h1>
            <div>
                <a href="/admin/forms/{slug}" class="btn">Edit Form</a>
            </div>
        </div>
        <table>
            <thead>
                <tr>
                    <th>ID</th>
                    {header_cells}
                    <th>Date</th>
                </tr>
            </thead>
            <tbody>
                {rows}
            </tbody>
        </table>
    </div>
</body>
</html>"##,
        form_name = html_escape(&form.name),
        slug = html_escape(&form.slug),
        header_cells = header_cells,
        rows = rows
    )
}

pub fn render_form(
    form: &FormConfig,
    inline_css: Option<&str>,
    css_url: Option<&str>,
    base_url: &str,
    is_htmx: bool,
) -> String {
    if is_htmx {
        render_form_fragment(form, base_url)
    } else {
        render_form_html(form, inline_css, css_url, base_url)
    }
}

fn render_form_html(
    form: &FormConfig,
    inline_css: Option<&str>,
    css_url: Option<&str>,
    base_url: &str,
) -> String {
    let s = &form.style;
    let has_file = form
        .fields
        .iter()
        .any(|f| matches!(f.field_type, FieldType::File));

    let css_link = css_url
        .map(|url| format!(r#"<link rel="stylesheet" href="{}">"#, html_escape(url)))
        .unwrap_or_default();
    let query_css = inline_css.unwrap_or_default();

    let title_html = if s.show_title {
        format!("<h1>{}</h1>", html_escape(&form.title))
    } else {
        String::new()
    };

    let fields_html: String = form
        .fields
        .iter()
        .map(|f| {
            let input = match f.field_type {
                FieldType::Text => format!(
                    r#"<input type="text" id="{id}" name="{id}" {req} placeholder="{ph}">"#,
                    id = html_escape(&f.id),
                    req = if f.required { "required" } else { "" },
                    ph = html_escape(f.placeholder.as_deref().unwrap_or(""))
                ),
                FieldType::Email => format!(
                    r#"<input type="email" id="{id}" name="{id}" {req} placeholder="{ph}">"#,
                    id = html_escape(&f.id),
                    req = if f.required { "required" } else { "" },
                    ph = html_escape(f.placeholder.as_deref().unwrap_or(""))
                ),
                FieldType::Mobile | FieldType::Phone => format!(
                    r#"<input type="tel" id="{id}" name="{id}" {req} placeholder="{ph}">"#,
                    id = html_escape(&f.id),
                    req = if f.required { "required" } else { "" },
                    ph = html_escape(f.placeholder.as_deref().unwrap_or(""))
                ),
                FieldType::LongText => format!(
                    r#"<textarea id="{id}" name="{id}" {req} placeholder="{ph}"></textarea>"#,
                    id = html_escape(&f.id),
                    req = if f.required { "required" } else { "" },
                    ph = html_escape(f.placeholder.as_deref().unwrap_or(""))
                ),
                FieldType::File => format!(
                    r#"<input type="file" id="{id}" name="{id}" {req}>"#,
                    id = html_escape(&f.id),
                    req = if f.required { "required" } else { "" }
                ),
            };
            format!(
                r#"<div class="form-group"><label for="{id}">{label}</label>{input}</div>"#,
                id = html_escape(&f.id),
                label = html_escape(&f.label),
                input = input
            )
        })
        .collect();

    let enctype = if has_file {
        r#" enctype="multipart/form-data""#
    } else {
        ""
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    {css_link}
    <style>
        *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
        :root {{
            --cf-font-family: {font_family};
            --cf-font-size: {font_size};
            --cf-text-color: {text_color};
            --cf-bg-color: {bg_color};
            --cf-form-bg: {form_bg};
            --cf-border-color: {border_color};
            --cf-border-radius: {border_radius};
            --cf-primary-color: {primary_color};
            --cf-input-padding: {input_padding};
        }}
        body {{
            font-family: var(--cf-font-family);
            font-size: var(--cf-font-size);
            color: var(--cf-text-color);
            max-width: 600px;
            margin: 0 auto;
            padding: 2rem;
            background: var(--cf-bg-color);
        }}
        h1 {{ margin-bottom: 1.5rem; }}
        .contact-form {{ background: var(--cf-form-bg); padding: 2rem; border-radius: var(--cf-border-radius); }}
        .form-group {{ margin-bottom: 1rem; }}
        label {{ display: block; margin-bottom: 0.5rem; font-weight: 500; }}
        input, textarea {{ width: 100%; padding: var(--cf-input-padding); border: 1px solid var(--cf-border-color); border-radius: var(--cf-border-radius); }}
        textarea {{ min-height: 120px; }}
        button {{ background: var(--cf-primary-color); color: white; padding: var(--cf-input-padding) 1.5rem; border: none; border-radius: var(--cf-border-radius); cursor: pointer; width: 100%; }}
        .success {{ background: #d4edda; color: #155724; padding: 2rem; border-radius: var(--cf-border-radius); text-align: center; }}
        .error {{ background: #f8d7da; color: #721c24; padding: 1rem; border-radius: var(--cf-border-radius); margin-bottom: 1rem; }}
        {custom_css}
        {query_css}
    </style>
</head>
<body>
    {title_html}
    <div id="form-container">
        <form class="contact-form" hx-post="{base_url}/f/{slug}/submit" hx-target="#form-container" hx-swap="innerHTML"{enctype}>
            {fields_html}
            <button type="submit">{submit_button_text}</button>
        </form>
    </div>
</body>
</html>"##,
        title = html_escape(&form.title),
        slug = html_escape(&form.slug),
        fields_html = fields_html,
        submit_button_text = html_escape(&form.submit_button_text),
        enctype = enctype,
        font_family = html_escape(&s.font_family),
        font_size = html_escape(&s.font_size),
        text_color = html_escape(&s.text_color),
        bg_color = html_escape(&s.bg_color),
        form_bg = html_escape(&s.form_bg),
        border_color = html_escape(&s.border_color),
        border_radius = html_escape(&s.border_radius),
        primary_color = html_escape(&s.primary_color),
        input_padding = html_escape(&s.input_padding),
        custom_css = s.custom_css,
        css_link = css_link,
        query_css = query_css,
        title_html = title_html,
        base_url = html_escape(base_url),
    )
}

fn render_form_fragment(form: &FormConfig, base_url: &str) -> String {
    let s = &form.style;
    let has_file = form
        .fields
        .iter()
        .any(|f| matches!(f.field_type, FieldType::File));

    let fields_html: String = form
        .fields
        .iter()
        .map(|f| {
            let input = match f.field_type {
                FieldType::Text => format!(
                    r#"<input type="text" id="{id}" name="{id}" {req} placeholder="{ph}">"#,
                    id = html_escape(&f.id),
                    req = if f.required { "required" } else { "" },
                    ph = html_escape(f.placeholder.as_deref().unwrap_or(""))
                ),
                FieldType::Email => format!(
                    r#"<input type="email" id="{id}" name="{id}" {req} placeholder="{ph}">"#,
                    id = html_escape(&f.id),
                    req = if f.required { "required" } else { "" },
                    ph = html_escape(f.placeholder.as_deref().unwrap_or(""))
                ),
                FieldType::Mobile | FieldType::Phone => format!(
                    r#"<input type="tel" id="{id}" name="{id}" {req} placeholder="{ph}">"#,
                    id = html_escape(&f.id),
                    req = if f.required { "required" } else { "" },
                    ph = html_escape(f.placeholder.as_deref().unwrap_or(""))
                ),
                FieldType::LongText => format!(
                    r#"<textarea id="{id}" name="{id}" {req} placeholder="{ph}"></textarea>"#,
                    id = html_escape(&f.id),
                    req = if f.required { "required" } else { "" },
                    ph = html_escape(f.placeholder.as_deref().unwrap_or(""))
                ),
                FieldType::File => format!(
                    r#"<input type="file" id="{id}" name="{id}" {req}>"#,
                    id = html_escape(&f.id),
                    req = if f.required { "required" } else { "" }
                ),
            };
            format!(
                r#"<div class="form-group"><label for="{id}">{label}</label>{input}</div>"#,
                id = html_escape(&f.id),
                label = html_escape(&f.label),
                input = input
            )
        })
        .collect();

    let enctype = if has_file {
        r#" enctype="multipart/form-data""#
    } else {
        ""
    };

    let title_html = if s.show_title {
        format!("<h1>{}</h1>", html_escape(&form.title))
    } else {
        String::new()
    };

    format!(
        r##"{title_html}
    <div id="form-container">
        <form class="contact-form" hx-post="{base_url}/f/{slug}/submit" hx-target="#form-container" hx-swap="innerHTML"{enctype}>
            {fields_html}
            <button type="submit">{submit_button_text}</button>
        </form>
    </div>"##,
        title_html = title_html,
        slug = html_escape(&form.slug),
        fields_html = fields_html,
        submit_button_text = html_escape(&form.submit_button_text),
        enctype = enctype,
        base_url = html_escape(base_url),
    )
}

pub fn form_success_html(message: &str, slug: &str, base_url: &str) -> String {
    format!(
        r##"<div class="success" hx-get="{base_url}/f/{slug}" hx-select="#form-container form" hx-swap="outerHTML" hx-trigger="load delay:3s">{message}</div>"##,
        message = html_escape(message),
        slug = html_escape(slug),
        base_url = html_escape(base_url)
    )
}

pub fn form_error_html(message: &str) -> String {
    format!(
        r##"<div class="error"><strong>Error:</strong> {}</div>"##,
        html_escape(message)
    )
}

pub fn format_digest_email(form: &FormConfig, submissions: &[Submission]) -> String {
    let mut body = format!(
        "New submissions for form: {}\n\nYou have {} new response(s) since the last digest.\n\n",
        form.name,
        submissions.len()
    );

    for (i, sub) in submissions.iter().enumerate() {
        body.push_str(&format!(
            "--- Response #{} ({}) ---\n",
            i + 1,
            sub.created_at
        ));
        for field in &form.fields {
            let value = sub
                .fields_data
                .get(&field.id)
                .map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    _ => v.to_string(),
                })
                .unwrap_or_else(|| "-".to_string());
            body.push_str(&format!("{}: {}\n", field.label, value));
        }
        body.push('\n');
    }

    body
}
