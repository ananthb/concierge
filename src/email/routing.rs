use crate::types::RoutingRule;

/// Case-insensitive glob matching.
/// Supports `*` (any sequence) and `?` (single char).
pub fn glob_match(pattern: &str, value: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let value = value.to_lowercase();
    glob_match_bytes(pattern.as_bytes(), value.as_bytes())
}

fn glob_match_bytes(pattern: &[u8], value: &[u8]) -> bool {
    let mut pi = 0;
    let mut vi = 0;
    let mut star_pi = None;
    let mut star_vi = 0;

    while vi < value.len() {
        if pi < pattern.len() && (pattern[pi] == b'?' || pattern[pi] == value[vi]) {
            pi += 1;
            vi += 1;
        } else if pi < pattern.len() && pattern[pi] == b'*' {
            star_pi = Some(pi);
            star_vi = vi;
            pi += 1;
        } else if let Some(sp) = star_pi {
            pi = sp + 1;
            star_vi += 1;
            vi = star_vi;
        } else {
            return false;
        }
    }

    while pi < pattern.len() && pattern[pi] == b'*' {
        pi += 1;
    }

    pi == pattern.len()
}

/// Check if an email matches a rule's criteria. All non-None fields must match (AND logic).
pub fn matches_rule(
    rule: &RoutingRule,
    from: &str,
    to: &str,
    subject: &str,
    has_attachment: bool,
    body: &str,
) -> bool {
    if !rule.enabled {
        return false;
    }

    if let Some(ref pat) = rule.criteria.from_pattern {
        if !glob_match(pat, from) {
            return false;
        }
    }
    if let Some(ref pat) = rule.criteria.to_pattern {
        if !glob_match(pat, to) {
            return false;
        }
    }
    if let Some(ref pat) = rule.criteria.subject_pattern {
        if !glob_match(pat, subject) {
            return false;
        }
    }
    if let Some(want_attachment) = rule.criteria.has_attachment {
        if has_attachment != want_attachment {
            return false;
        }
    }
    if let Some(ref pat) = rule.criteria.body_pattern {
        if !glob_match(pat, body) {
            return false;
        }
    }

    true
}

/// Find the highest-priority matching rule. Rules are assumed sorted by priority ascending;
/// the last matching rule wins (highest priority).
pub fn find_matching_rule<'a>(
    rules: &'a [RoutingRule],
    from: &str,
    to: &str,
    subject: &str,
    has_attachment: bool,
    body: &str,
) -> Option<&'a RoutingRule> {
    rules
        .iter()
        .filter(|r| matches_rule(r, from, to, subject, has_attachment, body))
        .last()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EmailAction, MatchCriteria};

    // ---- glob_match tests ----

    #[test]
    fn glob_exact_match() {
        assert!(glob_match("hello", "hello"));
        assert!(!glob_match("hello", "world"));
    }

    #[test]
    fn glob_case_insensitive() {
        assert!(glob_match("Hello", "hello"));
        assert!(glob_match("HELLO", "hello"));
        assert!(glob_match("hello", "HELLO"));
    }

    #[test]
    fn glob_star_matches_any() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*", ""));
        assert!(glob_match("hello*", "hello world"));
        assert!(glob_match("*world", "hello world"));
        assert!(glob_match("*llo*", "hello world"));
        assert!(glob_match("*@example.com", "user@example.com"));
        assert!(!glob_match("*@example.com", "user@other.com"));
    }

    #[test]
    fn glob_question_mark() {
        assert!(glob_match("h?llo", "hello"));
        assert!(glob_match("h?llo", "hallo"));
        assert!(!glob_match("h?llo", "hllo"));
    }

    #[test]
    fn glob_complex_patterns() {
        assert!(glob_match("*@*.example.com", "user@sub.example.com"));
        assert!(glob_match("*invoice*", "Your invoice #123 is ready"));
        assert!(glob_match(
            "support+*@example.com",
            "support+billing@example.com"
        ));
        assert!(!glob_match("support+*@example.com", "info@example.com"));
    }

    #[test]
    fn glob_empty() {
        assert!(glob_match("", ""));
        assert!(!glob_match("", "something"));
        assert!(glob_match("*", ""));
    }

    // ---- rule matching tests ----

    fn make_rule(
        priority: i32,
        from: Option<&str>,
        to: Option<&str>,
        subject: Option<&str>,
        has_attachment: Option<bool>,
    ) -> RoutingRule {
        RoutingRule {
            id: format!("rule-{priority}"),
            domain: "example.com".into(),
            name: format!("Rule {priority}"),
            priority,
            enabled: true,
            criteria: MatchCriteria {
                from_pattern: from.map(String::from),
                to_pattern: to.map(String::from),
                subject_pattern: subject.map(String::from),
                has_attachment,
                body_pattern: None,
            },
            action: EmailAction::Drop,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn catch_all_rule_matches_everything() {
        let rule = make_rule(0, None, None, None, None);
        assert!(matches_rule(
            &rule,
            "anyone@any.com",
            "alias@example.com",
            "Any subject",
            false,
            "body"
        ));
    }

    #[test]
    fn from_pattern_filters() {
        let rule = make_rule(10, Some("*@newsletter.com"), None, None, None);
        assert!(matches_rule(
            &rule,
            "noreply@newsletter.com",
            "me@example.com",
            "Weekly digest",
            false,
            ""
        ));
        assert!(!matches_rule(
            &rule,
            "alice@other.com",
            "me@example.com",
            "Hi",
            false,
            ""
        ));
    }

    #[test]
    fn multiple_criteria_and_logic() {
        let rule = make_rule(20, Some("*@shop.com"), None, Some("*invoice*"), Some(true));
        // All match
        assert!(matches_rule(
            &rule,
            "billing@shop.com",
            "me@example.com",
            "Your invoice #42",
            true,
            ""
        ));
        // From doesn't match
        assert!(!matches_rule(
            &rule,
            "billing@other.com",
            "me@example.com",
            "Your invoice #42",
            true,
            ""
        ));
        // Subject doesn't match
        assert!(!matches_rule(
            &rule,
            "billing@shop.com",
            "me@example.com",
            "Order confirmation",
            true,
            ""
        ));
        // No attachment
        assert!(!matches_rule(
            &rule,
            "billing@shop.com",
            "me@example.com",
            "Your invoice #42",
            false,
            ""
        ));
    }

    #[test]
    fn disabled_rule_never_matches() {
        let mut rule = make_rule(10, None, None, None, None);
        rule.enabled = false;
        assert!(!matches_rule(&rule, "a@b.com", "c@d.com", "sub", false, ""));
    }

    #[test]
    fn highest_priority_wins() {
        let rules = vec![
            make_rule(0, None, None, None, None), // catch-all
            make_rule(10, Some("*@newsletter.com"), None, None, None), // newsletter
            make_rule(20, Some("spam@newsletter.com"), None, None, None), // specific sender
        ];

        // Generic sender → catch-all (priority 0)
        let matched = find_matching_rule(&rules, "alice@other.com", "me@x.com", "", false, "");
        assert_eq!(matched.unwrap().priority, 0);

        // Newsletter sender → newsletter rule (priority 10)
        let matched = find_matching_rule(&rules, "news@newsletter.com", "me@x.com", "", false, "");
        assert_eq!(matched.unwrap().priority, 10);

        // Specific spam sender → specific rule (priority 20)
        let matched = find_matching_rule(&rules, "spam@newsletter.com", "me@x.com", "", false, "");
        assert_eq!(matched.unwrap().priority, 20);
    }

    #[test]
    fn no_rules_returns_none() {
        let rules: Vec<RoutingRule> = vec![];
        assert!(find_matching_rule(&rules, "a@b.com", "c@d.com", "", false, "").is_none());
    }

    #[test]
    fn body_pattern_matching() {
        let mut rule = make_rule(10, None, None, None, None);
        rule.criteria.body_pattern = Some("*unsubscribe*".into());

        assert!(matches_rule(
            &rule,
            "a@b.com",
            "c@d.com",
            "",
            false,
            "Click here to unsubscribe from this list"
        ));
        assert!(!matches_rule(
            &rule,
            "a@b.com",
            "c@d.com",
            "",
            false,
            "Normal email content"
        ));
    }
}
