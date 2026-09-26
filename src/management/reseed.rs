//! Destructive schema reseed: clear every table, then re-apply the
//! canonical schema from `migrations/0001_create_schema.sql`.
//!
//! Why this lives in the Worker rather than in a wrangler script.
//! `migrations/0001_create_schema.sql` is a single canonical file that is
//! edited in place while the schema is pre-GA, and
//! `wrangler d1 migrations apply` cannot express that: it skips a
//! migration it has already run, and every `CREATE` is `IF NOT EXISTS`,
//! so re-running the file against a live database is a no-op that leaves
//! removed columns in place. The tables have to go first. Doing it from
//! here means `git push` is the whole deploy: no Cloudflare credentials
//! on anyone's laptop, because the Worker already holds the `DB` binding.
//!
//! Two independent gates, both required:
//!
//!   1. The route lives under `/manage`, which is Cloudflare
//!      Access-protected. Reaching it at all means an operator identity
//!      (or, locally, the dev bypass in [`crate::dev_bypass`]).
//!   2. `ALLOW_SCHEMA_RESEED` must equal the database's own name. Not `1`,
//!      not `true` — the name, so it records *which* database it licenses
//!      and cannot be set by reflex.
//!
//! The flag lives in `wrangler.toml` `[vars]`, commented out. Uncomment,
//! push, reseed, re-comment, push. Not a dashboard variable and not
//! Terraform: `tf/cloudflare/sites/workers.tf` in platform owns what the
//! Workers are reached through and bind, never the scripts or their vars,
//! so wrangler.toml is the one owner. Licensing and revoking are then both
//! commits.
//!
//! The request body must name the same database, so a stray click cannot
//! fire it during the window where the variable is set.
//!
//! `scripts/recreate-db.sh` does the same job from a laptop with an
//! authenticated wrangler. It stays as the break-glass path: a schema
//! change that stops the Worker booting is exactly when this endpoint is
//! unreachable.

use worker::*;

/// The canonical schema, compiled in, so the file the repo edits is the
/// file this applies.
pub const SCHEMA_SQL: &str = include_str!("../../migrations/0001_create_schema.sql");

/// Env var that licenses a reseed. Its value must equal the database name.
const RESEED_VAR: &str = "ALLOW_SCHEMA_RESEED";

/// Whether this table belongs to the application rather than to SQLite or
/// D1 itself. `_cf_*` is D1 internal and refuses to be removed; `sqlite_*`
/// is SQLite's own. `d1_migrations` is wrangler's bookkeeping and *must*
/// be included, or a later `migrations apply` still believes 0001 has run.
fn is_app_table(name: &str) -> bool {
    !name.starts_with("sqlite_") && !name.starts_with("_cf_")
}

/// Split a SQL script into individually executable statements.
///
/// Hand-rolled because this schema cannot be split on `;` naively: the
/// archetype seed rows contain semicolons *inside* quoted strings
/// ("…outside business hours; we'll respond when we're back."), and `--`
/// comments appear throughout. Tracks single-quote state, treats SQL's
/// doubled `''` as an escaped quote, and breaks only on a semicolon found
/// outside a string.
///
/// Deliberately does not understand `BEGIN … END` bodies (triggers), which
/// carry their own statement terminators. This schema has none; if one is
/// ever added, this needs to learn about them.
pub fn split_statements(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_string = false;
    let mut chars = sql.chars().peekable();

    while let Some(c) = chars.next() {
        if in_string {
            cur.push(c);
            if c == '\'' {
                // A doubled quote is an escaped quote, not a terminator.
                if chars.peek() == Some(&'\'') {
                    cur.push(chars.next().unwrap());
                } else {
                    in_string = false;
                }
            }
            continue;
        }
        match c {
            '\'' => {
                in_string = true;
                cur.push(c);
            }
            '-' if chars.peek() == Some(&'-') => {
                // Line comment: discard to end of line, keeping the
                // newline so tokens either side stay separated.
                for c2 in chars.by_ref() {
                    if c2 == '\n' {
                        break;
                    }
                }
                cur.push('\n');
            }
            ';' => {
                if !cur.trim().is_empty() {
                    out.push(cur.trim().to_string());
                }
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// What a reseed did, reported back to the operator.
pub struct ReseedReport {
    pub cleared: Vec<String>,
    pub statements_run: usize,
}

/// The licensed database name, when the gate variable is set.
fn licensed_database(env: &Env) -> Option<String> {
    env.var(RESEED_VAR)
        .ok()
        .map(|v| v.to_string().trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Clear every application table, then apply [`SCHEMA_SQL`].
///
/// The caller owns the gates; this is only the mechanism.
pub async fn reseed(db: &D1Database) -> Result<ReseedReport> {
    let rows = db
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
        .all()
        .await?
        .results::<serde_json::Value>()?;

    let mut cleared = Vec::new();
    for row in rows {
        let Some(name) = row.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        if !is_app_table(name) {
            continue;
        }
        // Names come from sqlite_master, never from request input, so they
        // cannot smuggle SQL — quoted anyway so a name needing it works.
        db.exec(&format!("DROP TABLE IF EXISTS \"{name}\";"))
            .await?;
        cleared.push(name.to_string());
    }

    let statements = split_statements(SCHEMA_SQL);
    for stmt in &statements {
        db.prepare(stmt).run().await?;
    }

    Ok(ReseedReport {
        cleared,
        statements_run: statements.len(),
    })
}

/// `POST /manage/reseed`. Gates are described in the module docs.
pub async fn handle_reseed(
    mut req: Request,
    env: &Env,
    db: &D1Database,
    actor_email: &str,
) -> Result<Response> {
    let Some(licensed) = licensed_database(env) else {
        return Response::from_html(format!(
            r#"<div class="error">Reseed is not licensed. Uncomment <code>{RESEED_VAR}</code> in <code>wrangler.toml</code> <code>[vars]</code>, push, then retry.</div>"#
        ));
    };

    let form: serde_json::Value = req.json().await.unwrap_or(serde_json::Value::Null);
    let confirm = form
        .get("confirm")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    if confirm != licensed {
        return Response::from_html(
            r#"<div class="error">Type the database name to confirm. It must match the licensing variable.</div>"#,
        );
    }

    console_log!("SCHEMA RESEED starting, requested by {actor_email}");
    let report = match reseed(db).await {
        Ok(r) => r,
        Err(e) => {
            console_log!("SCHEMA RESEED FAILED: {e:?}");
            return Response::from_html(format!(
                r#"<div class="error">Reseed failed: {}. The database may be partially cleared — check /manage before retrying.</div>"#,
                crate::helpers::html_escape(&e.to_string())
            ));
        }
    };
    console_log!(
        "SCHEMA RESEED done: cleared {} tables, ran {} statements",
        report.cleared.len(),
        report.statements_run
    );

    // Audited after the fact by necessity: audit_log is one of the tables
    // this clears, so an entry written first would not survive.
    if let Err(e) = crate::management::audit::log_action(
        db,
        actor_email,
        "schema_reseed",
        "database",
        Some(&licensed),
        Some(&serde_json::json!({
            "cleared": report.cleared,
            "statements_run": report.statements_run,
        })),
    )
    .await
    {
        console_log!("Reseed audit entry failed: {e:?}");
    }

    Response::from_html(format!(
        r#"<div class="success">Reseeded <strong>{db_name}</strong>: cleared {n} tables, applied {s} statements. Re-comment <code>{RESEED_VAR}</code> in <code>wrangler.toml</code> and push.</div>"#,
        db_name = crate::helpers::html_escape(&licensed),
        n = report.cleared.len(),
        s = report.statements_run,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_statement_terminators() {
        let out = split_statements("CREATE TABLE a (id TEXT);\nCREATE TABLE b (id TEXT);");
        assert_eq!(out.len(), 2);
        assert!(out[0].starts_with("CREATE TABLE a"));
        assert!(out[1].starts_with("CREATE TABLE b"));
    }

    #[test]
    fn a_semicolon_inside_a_string_does_not_split() {
        // The real hazard: the archetype seed rows contain exactly this.
        let out = split_statements("INSERT INTO t VALUES ('closed; back soon');");
        assert_eq!(out.len(), 1, "split inside a quoted string: {out:?}");
        assert!(out[0].contains("closed; back soon"));
    }

    #[test]
    fn doubled_quotes_are_escapes_not_terminators() {
        // '' is SQL's escaped quote. Mishandling it flips the in-string
        // state and every later semicolon then splits in the wrong place.
        let out = split_statements("INSERT INTO t VALUES ('we''re shut; back Monday');\nSELECT 1;");
        assert_eq!(out.len(), 2, "{out:?}");
        assert!(out[0].contains("we''re shut; back Monday"));
        assert_eq!(out[1], "SELECT 1");
    }

    #[test]
    fn line_comments_are_stripped_including_ones_holding_semicolons() {
        let out = split_statements("-- drop this; and this\nSELECT 1;\nSELECT 2; -- trailing\n");
        assert_eq!(out, vec!["SELECT 1", "SELECT 2"], "{out:?}");
    }

    #[test]
    fn a_double_dash_inside_a_string_is_not_a_comment() {
        let out = split_statements("INSERT INTO t VALUES ('a--b');");
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("a--b"), "{out:?}");
    }

    #[test]
    fn trailing_statement_without_a_terminator_is_kept() {
        assert_eq!(split_statements("SELECT 1"), vec!["SELECT 1"]);
    }

    #[test]
    fn the_real_schema_splits_into_plausible_statements() {
        let out = split_statements(SCHEMA_SQL);
        // Every fragment must be executable SQL, not a comment scrap.
        for s in &out {
            let head = s.split_whitespace().next().unwrap_or("").to_uppercase();
            assert!(
                matches!(head.as_str(), "CREATE" | "INSERT"),
                "unexpected statement head {head:?} in: {}",
                &s[..s.len().min(80)]
            );
        }
        // Sanity on scale: tables + indexes + seeds.
        assert!(out.len() > 20, "only {} statements parsed", out.len());
        // And what this whole change removed must stay removed.
        let joined = out.join("\n");
        assert!(!joined.contains("email_address_extras_purchased"));
        assert!(!joined.contains("email_pack_size"));
        assert!(!joined.contains("address_price"));
    }

    #[test]
    fn sqlite_and_d1_internal_tables_are_left_alone() {
        assert!(!is_app_table("sqlite_sequence"));
        assert!(!is_app_table("_cf_KV"));
        // wrangler's bookkeeping must be included, or 0001 looks applied.
        assert!(is_app_table("d1_migrations"));
        assert!(is_app_table("tenants"));
    }
}
