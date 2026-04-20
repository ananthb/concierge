//! Admin templates for email routing

use crate::helpers::html_escape;
use crate::types::*;

use super::base::base_html;
use super::HASH;

fn action_label(action: &EmailAction) -> &'static str {
    match action {
        EmailAction::Drop => "Drop",
        EmailAction::Spam { .. } => "Spam",
        EmailAction::ForwardEmail { .. } => "Forward Email",
        EmailAction::ForwardDiscord { .. } => "Forward Discord",
        EmailAction::AiReply { .. } => "AI Reply",
    }
}

fn action_detail(action: &EmailAction) -> String {
    match action {
        EmailAction::Drop => String::new(),
        EmailAction::Spam { message } => message.clone().unwrap_or_default(),
        EmailAction::ForwardEmail { destination } => html_escape(destination),
        EmailAction::ForwardDiscord { channel_id } => {
            format!("Channel: {}", html_escape(channel_id))
        }
        EmailAction::AiReply {
            approval_channel_id,
            approval_email,
            ..
        } => {
            let mut parts = Vec::new();
            if let Some(ch) = approval_channel_id {
                parts.push(format!("Discord: {}", html_escape(ch)));
            }
            if let Some(em) = approval_email {
                parts.push(format!("Email: {}", html_escape(em)));
            }
            parts.join(", ")
        }
    }
}

fn criteria_summary(c: &MatchCriteria) -> String {
    let mut parts = Vec::new();
    if let Some(ref p) = c.from_pattern {
        parts.push(format!("From: {}", html_escape(p)));
    }
    if let Some(ref p) = c.to_pattern {
        parts.push(format!("To: {}", html_escape(p)));
    }
    if let Some(ref p) = c.subject_pattern {
        parts.push(format!("Subject: {}", html_escape(p)));
    }
    if let Some(true) = c.has_attachment {
        parts.push("Has attachment".into());
    }
    if let Some(ref p) = c.body_pattern {
        parts.push(format!("Body: {}", html_escape(p)));
    }
    if parts.is_empty() {
        "Catch-all".into()
    } else {
        parts.join("; ")
    }
}

pub fn email_dashboard_html(
    domains: &[EmailSubdomain],
    metrics: &[serde_json::Value],
    base_url: &str,
) -> String {
    let domain_rows: String = domains
        .iter()
        .map(|d| {
            format!(
                "<tr>
                    <td><a href=\"{base_url}/admin/email/domains/{domain}/rules\">{domain}</a></td>
                    <td>{action}</td>
                    <td>
                        <a href=\"{base_url}/admin/email/domains/{domain}/rules\" class=\"btn btn-sm\">Rules</a>
                        <button class=\"btn btn-sm btn-danger\" hx-delete=\"{base_url}/admin/email/domains/{domain}\" hx-confirm=\"Delete domain {domain} and all its rules?\" hx-target=\"closest tr\" hx-swap=\"outerHTML\">Delete</button>
                    </td>
                </tr>",
                base_url = base_url,
                domain = html_escape(&d.domain),
                action = action_label(&d.default_action),
            )
        })
        .collect();

    let domain_empty = if domains.is_empty() {
        "<tr><td colspan=\"3\" class=\"text-muted\">No domains configured.</td></tr>"
    } else {
        ""
    };

    let metrics_html: String = metrics
        .iter()
        .map(|m| {
            let action = m.get("action_type").and_then(|v| v.as_str()).unwrap_or("?");
            let total = m.get("total").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
            format!("<li><strong>{action}</strong>: {total}</li>")
        })
        .collect();

    let content = format!(
        "<p><a href=\"{base_url}/admin\">&larr; Back to Dashboard</a></p>
        <div style=\"display: flex; justify-content: space-between; align-items: center;\">
            <h1>Email Routing</h1>
            <div>
                <a href=\"{base_url}/admin/email/log\" class=\"btn btn-secondary btn-sm\">View Log</a>
                <a href=\"{base_url}/admin/email/settings\" class=\"btn btn-secondary btn-sm\">Settings</a>
            </div>
        </div>

        <div class=\"card\">
            <h2 style=\"margin-bottom: 0.5rem;\">Last 7 Days</h2>
            {metrics_section}
        </div>

        <div class=\"card\">
            <div style=\"display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;\">
                <h2>Domains</h2>
            </div>
            <table>
                <thead><tr><th>Domain</th><th>Default Action</th><th>Actions</th></tr></thead>
                <tbody>{domain_rows}{domain_empty}</tbody>
            </table>

            <div id=\"toast\" style=\"margin-top: 1rem;\"></div>
            <h3 style=\"margin-top: 1.5rem;\">Add Domain</h3>
            <form hx-post=\"{base_url}/admin/email/domains\" hx-target=\"{hash}toast\" hx-swap=\"innerHTML\"
                  style=\"display: flex; gap: 0.5rem; margin-top: 0.5rem;\">
                <input type=\"text\" name=\"domain\" placeholder=\"example.com\" required style=\"flex: 1; padding: 0.5rem; border: 1px solid var(--border); border-radius: var(--cal-border-radius); background: var(--bg-card); color: var(--cal-text);\">
                <button type=\"submit\" class=\"btn\">Add</button>
            </form>
        </div>",
        base_url = base_url,
        domain_rows = domain_rows,
        domain_empty = domain_empty,
        hash = HASH,
        metrics_section = if metrics_html.is_empty() {
            "<p class=\"text-muted\">No email activity yet.</p>".to_string()
        } else {
            format!("<ul style=\"list-style: none; padding: 0;\">{metrics_html}</ul>")
        },
    );

    base_html("Email Routing - Concierge", &content)
}

pub fn email_rules_html(domain: &str, rules: &[RoutingRule], base_url: &str) -> String {
    let rule_rows: String = rules
        .iter()
        .map(|r| {
            let enabled_badge = if r.enabled {
                "<span style=\"color: var(--success-text);\">On</span>"
            } else {
                "<span class=\"text-muted\">Off</span>"
            };
            format!(
                "<tr>
                    <td>{priority}</td>
                    <td><a href=\"{base_url}/admin/email/domains/{domain}/rules/{id}\">{name}</a></td>
                    <td><small>{criteria}</small></td>
                    <td>{action} <small class=\"text-muted\">{detail}</small></td>
                    <td>{enabled_badge}</td>
                    <td>
                        <button class=\"btn btn-sm\" hx-post=\"{base_url}/admin/email/domains/{domain}/rules/{id}/toggle\">Toggle</button>
                        <button class=\"btn btn-sm btn-danger\" hx-delete=\"{base_url}/admin/email/domains/{domain}/rules/{id}\" hx-confirm=\"Delete this rule?\" hx-target=\"closest tr\" hx-swap=\"outerHTML\">Delete</button>
                    </td>
                </tr>",
                base_url = base_url,
                domain = html_escape(domain),
                id = html_escape(&r.id),
                name = html_escape(&r.name),
                priority = r.priority,
                criteria = criteria_summary(&r.criteria),
                action = action_label(&r.action),
                detail = action_detail(&r.action),
                enabled_badge = enabled_badge,
            )
        })
        .collect();

    let empty = if rules.is_empty() {
        "<tr><td colspan=\"6\" class=\"text-muted\">No rules configured. Add one below.</td></tr>"
    } else {
        ""
    };

    let content = format!(
        "<p><a href=\"{base_url}/admin/email\">&larr; Back to Email Routing</a></p>
        <h1>Rules for {domain}</h1>

        <div class=\"card\">
            <table>
                <thead><tr><th>Priority</th><th>Name</th><th>Criteria</th><th>Action</th><th>Status</th><th></th></tr></thead>
                <tbody>{rule_rows}{empty}</tbody>
            </table>
        </div>

        <div class=\"card\">
            <h2>Add Rule</h2>
            <div id=\"toast\"></div>
            {rule_form}
        </div>",
        base_url = base_url,
        domain = html_escape(domain),
        rule_rows = rule_rows,
        empty = empty,
        rule_form = rule_form_html(domain, None, base_url),
    );

    base_html(&format!("Rules: {} - Concierge", domain), &content)
}

pub fn email_rule_edit_html(domain: &str, rule: &RoutingRule, base_url: &str) -> String {
    let content = format!(
        "<p><a href=\"{base_url}/admin/email/domains/{domain}/rules\">&larr; Back to Rules</a></p>
        <h1>Edit Rule: {name}</h1>
        <div class=\"card\">
            <div id=\"toast\"></div>
            {form}
        </div>",
        base_url = base_url,
        domain = html_escape(domain),
        name = html_escape(&rule.name),
        form = rule_form_html(domain, Some(rule), base_url),
    );

    base_html(&format!("Edit Rule - Concierge"), &content)
}

fn rule_form_html(domain: &str, rule: Option<&RoutingRule>, base_url: &str) -> String {
    let (method, url) = match rule {
        Some(r) => (
            "hx-put",
            format!(
                "{base_url}/admin/email/domains/{}/rules/{}",
                html_escape(domain),
                html_escape(&r.id)
            ),
        ),
        None => (
            "hx-post",
            format!(
                "{base_url}/admin/email/domains/{}/rules",
                html_escape(domain)
            ),
        ),
    };

    let name = rule.map(|r| html_escape(&r.name)).unwrap_or_default();
    let priority = rule.map(|r| r.priority.to_string()).unwrap_or("0".into());
    let enabled_checked = if rule.map(|r| r.enabled).unwrap_or(true) {
        " checked"
    } else {
        ""
    };

    let from_pattern = rule
        .and_then(|r| r.criteria.from_pattern.as_deref())
        .map(html_escape)
        .unwrap_or_default();
    let to_pattern = rule
        .and_then(|r| r.criteria.to_pattern.as_deref())
        .map(html_escape)
        .unwrap_or_default();
    let subject_pattern = rule
        .and_then(|r| r.criteria.subject_pattern.as_deref())
        .map(html_escape)
        .unwrap_or_default();
    let body_pattern = rule
        .and_then(|r| r.criteria.body_pattern.as_deref())
        .map(html_escape)
        .unwrap_or_default();
    let has_attachment_checked = if rule
        .and_then(|r| r.criteria.has_attachment)
        .unwrap_or(false)
    {
        " checked"
    } else {
        ""
    };

    let (
        action_type,
        destination,
        channel_id,
        spam_message,
        system_prompt,
        approval_channel_id,
        approval_email,
    ) = match rule.map(|r| &r.action) {
        Some(EmailAction::Drop) | None => ("drop", "", "", "", "", "", ""),
        Some(EmailAction::Spam { message }) => {
            ("spam", "", "", message.as_deref().unwrap_or(""), "", "", "")
        }
        Some(EmailAction::ForwardEmail { destination }) => {
            ("forward_email", destination.as_str(), "", "", "", "", "")
        }
        Some(EmailAction::ForwardDiscord { channel_id }) => {
            ("forward_discord", "", channel_id.as_str(), "", "", "", "")
        }
        Some(EmailAction::AiReply {
            system_prompt,
            approval_channel_id,
            approval_email,
        }) => (
            "ai_reply",
            "",
            "",
            "",
            system_prompt.as_deref().unwrap_or(""),
            approval_channel_id.as_deref().unwrap_or(""),
            approval_email.as_deref().unwrap_or(""),
        ),
    };

    let action_options = [
        ("drop", "Drop"),
        ("spam", "Reject (Spam)"),
        ("forward_email", "Forward to Email"),
        ("forward_discord", "Forward to Discord"),
        ("ai_reply", "AI Reply"),
    ];

    let action_select: String = action_options
        .iter()
        .map(|(val, label)| {
            let selected = if *val == action_type { " selected" } else { "" };
            format!("<option value=\"{val}\"{selected}>{label}</option>")
        })
        .collect();

    format!(
        "<form {method}=\"{url}\" hx-target=\"{hash}toast\" hx-swap=\"innerHTML\">
            <div class=\"form-group\">
                <label>Name</label>
                <input type=\"text\" name=\"name\" value=\"{name}\" placeholder=\"Newsletter filter\" required>
            </div>
            <div style=\"display: flex; gap: 1rem;\">
                <div class=\"form-group\" style=\"flex: 1;\">
                    <label>Priority</label>
                    <input type=\"number\" name=\"priority\" value=\"{priority}\" style=\"width: 100%;\">
                </div>
                <div class=\"form-group\" style=\"flex: 1;\">
                    <label><input type=\"checkbox\" name=\"enabled\" value=\"true\"{enabled_checked}> Enabled</label>
                </div>
            </div>

            <h3 style=\"margin: 1rem 0 0.5rem;\">Match Criteria</h3>
            <p class=\"muted\" style=\"margin-bottom:12px\"><small>Patterns: <code>*</code> matches anything, <code>?</code> matches one character. Example: <code>*@newsletter.com</code> matches all emails from newsletter.com</small></p>
            <div class=\"form-group\">
                <label>From</label>
                <input type=\"text\" name=\"from_pattern\" value=\"{from_pattern}\" placeholder=\"*@newsletter.com\">
            </div>
            <div class=\"form-group\">
                <label>To</label>
                <input type=\"text\" name=\"to_pattern\" value=\"{to_pattern}\" placeholder=\"support+*@example.com\">
            </div>
            <div class=\"form-group\">
                <label>Subject</label>
                <input type=\"text\" name=\"subject_pattern\" value=\"{subject_pattern}\" placeholder=\"*invoice*\">
            </div>
            <div class=\"form-group\">
                <label>Body contains</label>
                <input type=\"text\" name=\"body_pattern\" value=\"{body_pattern}\" placeholder=\"*unsubscribe*\">
            </div>
            <div class=\"form-group\">
                <label><input type=\"checkbox\" name=\"has_attachment\" value=\"true\"{has_attachment_checked}> Has attachment</label>
            </div>

            <h3 style=\"margin: 1rem 0 0.5rem;\">Action</h3>
            <div class=\"form-group\">
                <label>Action type</label>
                <select name=\"action_type\" onchange=\"document.querySelectorAll('.action-fields').forEach(e => e.style.display='none'); var el=document.getElementById('action-'+this.value); if(el) el.style.display='block';\">
                    {action_select}
                </select>
            </div>

            <div id=\"action-spam\" class=\"action-fields\" style=\"display:{spam_display};\">
                <div class=\"form-group\">
                    <label>Reject message</label>
                    <input type=\"text\" name=\"spam_message\" value=\"{spam_message}\" placeholder=\"Rejected as spam\">
                </div>
            </div>
            <div id=\"action-forward_email\" class=\"action-fields\" style=\"display:{fwd_display};\">
                <div class=\"form-group\">
                    <label>Destination email</label>
                    <input type=\"email\" name=\"destination\" value=\"{destination}\" placeholder=\"me@gmail.com\">
                </div>
            </div>
            <div id=\"action-forward_discord\" class=\"action-fields\" style=\"display:{discord_display};\">
                <div class=\"form-group\">
                    <label>Discord channel ID</label>
                    <input type=\"text\" name=\"channel_id\" value=\"{channel_id}\" placeholder=\"123456789012345678\">
                </div>
            </div>
            <div id=\"action-ai_reply\" class=\"action-fields\" style=\"display:{ai_display};\">
                <div class=\"form-group\">
                    <label>System prompt</label>
                    <textarea name=\"system_prompt\" rows=\"3\" placeholder=\"You are a helpful assistant...\">{system_prompt}</textarea>
                </div>
                <div class=\"form-group\">
                    <label>Approval Discord channel ID</label>
                    <input type=\"text\" name=\"approval_channel_id\" value=\"{approval_channel_id}\" placeholder=\"Channel for approval\">
                </div>
                <div class=\"form-group\">
                    <label>Approval email</label>
                    <input type=\"email\" name=\"approval_email\" value=\"{approval_email}\" placeholder=\"Or send approval to email\">
                </div>
            </div>

            <div style=\"display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 1rem;\">
                <button type=\"submit\" class=\"btn\">Save</button>
            </div>
        </form>",
        method = method,
        url = url,
        hash = HASH,
        name = name,
        priority = priority,
        enabled_checked = enabled_checked,
        from_pattern = from_pattern,
        to_pattern = to_pattern,
        subject_pattern = subject_pattern,
        body_pattern = body_pattern,
        has_attachment_checked = has_attachment_checked,
        action_select = action_select,
        destination = html_escape(destination),
        channel_id = html_escape(channel_id),
        spam_message = html_escape(spam_message),
        system_prompt = html_escape(system_prompt),
        approval_channel_id = html_escape(approval_channel_id),
        approval_email = html_escape(approval_email),
        spam_display = if action_type == "spam" { "block" } else { "none" },
        fwd_display = if action_type == "forward_email" { "block" } else { "none" },
        discord_display = if action_type == "forward_discord" { "block" } else { "none" },
        ai_display = if action_type == "ai_reply" { "block" } else { "none" },
    )
}

pub fn email_log_html(log: &[serde_json::Value], base_url: &str) -> String {
    let rows: String = log
        .iter()
        .map(|entry| {
            let from = entry
                .get("from_email")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let to = entry.get("to_email").and_then(|v| v.as_str()).unwrap_or("");
            let subject = entry.get("subject").and_then(|v| v.as_str()).unwrap_or("");
            let action = entry
                .get("action_taken")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let domain = entry.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            let created = entry
                .get("created_at")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let error = entry
                .get("error_msg")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let status = if error.is_empty() {
                format!("<span style=\"color: var(--success-text);\">{action}</span>")
            } else {
                format!(
                    "<span style=\"color: var(--error-text);\">{action}: {error}</span>",
                    action = html_escape(action),
                    error = html_escape(error)
                )
            };

            format!(
                "<tr>
                    <td><small>{created}</small></td>
                    <td>{domain}</td>
                    <td><small>{from}</small></td>
                    <td><small>{to}</small></td>
                    <td><small>{subject}</small></td>
                    <td>{status}</td>
                </tr>",
                created = html_escape(created),
                domain = html_escape(domain),
                from = html_escape(from),
                to = html_escape(to),
                subject = html_escape(subject),
                status = status,
            )
        })
        .collect();

    let empty = if log.is_empty() {
        "<tr><td colspan=\"6\" class=\"text-muted\">No emails logged yet.</td></tr>"
    } else {
        ""
    };

    let content = format!(
        "<p><a href=\"{base_url}/admin/email\">&larr; Back to Email Routing</a></p>
        <h1>Email Log</h1>
        <div class=\"card\" style=\"overflow-x: auto;\">
            <table>
                <thead><tr><th>Time</th><th>Domain</th><th>From</th><th>To</th><th>Subject</th><th>Status</th></tr></thead>
                <tbody>{rows}{empty}</tbody>
            </table>
        </div>",
        base_url = base_url,
        rows = rows,
        empty = empty,
    );

    base_html("Email Log - Concierge", &content)
}

pub fn email_settings_html(discord_token: Option<&str>, base_url: &str) -> String {
    let token_value = discord_token.unwrap_or("");

    let content = format!(
        "<p><a href=\"{base_url}/admin/email\">&larr; Back to Email Routing</a></p>
        <h1>Email Settings</h1>
        <div class=\"card\">
            <div id=\"toast\"></div>
            <h2>Discord Bot</h2>
            <p class=\"text-muted\" style=\"margin-bottom: 1rem;\">Bot token for forwarding emails to Discord channels.</p>
            <form hx-put=\"{base_url}/admin/email/settings\" hx-target=\"{hash}toast\" hx-swap=\"innerHTML\">
                <div class=\"form-group\">
                    <label>Bot Token</label>
                    <input type=\"password\" name=\"discord_bot_token\" value=\"{token}\" placeholder=\"Bot token\" style=\"font-family: monospace;\">
                </div>
                <button type=\"submit\" class=\"btn\">Save</button>
            </form>
        </div>",
        base_url = base_url,
        hash = HASH,
        token = html_escape(token_value),
    );

    base_html("Email Settings - Concierge", &content)
}
