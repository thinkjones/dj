pub mod template;
pub mod targets;
pub mod translators;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternCategory {
    Shell,
    Edit,
    Read,
    WebSearch,
    WebFetch,
}

#[derive(Debug, Clone)]
pub struct ParsedPattern {
    pub category: PatternCategory,
    pub decision: Decision,
    pub pattern: String,        // normalized (see below)
    pub domain: Option<String>, // for WebFetch
    pub raw: String,            // original string from template array
}

impl ParsedPattern {
    /// True if Bash(X:*) form — prefix wildcard.
    /// Public lib API + exercised by tests; the binary target doesn't call it.
    #[allow(dead_code)]
    pub fn is_shell_prefix_wildcard(&self) -> bool {
        self.raw.starts_with("Bash(") && self.raw.ends_with(":*)")
    }
    /// True if Bash pattern has embedded * or | (not from :* suffix)
    pub fn is_shell_embedded_wildcard(&self) -> bool {
        if !self.raw.starts_with("Bash(") {
            return false;
        }
        if self.raw.ends_with(":*)") {
            return false;
        }
        // content inside Bash(...)
        let inner = &self.raw[5..self.raw.len() - 1];
        inner.contains('*') || inner.contains('|')
    }
    /// For Shell: the command prefix (e.g. "git", "npm test", "git push --force")
    pub fn shell_base_command(&self) -> Option<String> {
        if self.category != PatternCategory::Shell {
            return None;
        }
        let inner = &self.raw[5..self.raw.len() - 1]; // content inside Bash(...)
        if let Some(cmd) = inner.strip_suffix(":*") {
            Some(cmd.to_string())
        } else if inner.contains('*') || inner.contains('|') {
            // embedded wildcard: use first word as base
            Some(
                inner
                    .split_whitespace()
                    .next()
                    .unwrap_or(inner)
                    .to_string(),
            )
        } else {
            Some(inner.to_string())
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TemplatePermissions {
    pub raw_allow: Vec<String>,
    pub raw_deny: Vec<String>,
    pub allow: Vec<ParsedPattern>,
    pub deny: Vec<ParsedPattern>,
}

impl TemplatePermissions {
    pub fn has_any_deny(&self) -> bool {
        !self.deny.is_empty()
    }
    pub fn all_patterns(&self) -> impl Iterator<Item = &ParsedPattern> {
        self.allow.iter().chain(self.deny.iter())
    }
}

/// Parse a single pattern string into a ParsedPattern, given its decision.
/// Returns None if unrecognized (call site should emit stderr warning).
pub fn parse_pattern(raw: &str, decision: Decision) -> Option<ParsedPattern> {
    let raw = raw.trim();

    if raw == "WebSearch" {
        return Some(ParsedPattern {
            category: PatternCategory::WebSearch,
            decision,
            pattern: String::new(),
            domain: None,
            raw: raw.to_string(),
        });
    }

    if let Some(inner) = raw
        .strip_prefix("Bash(")
        .and_then(|s| s.strip_suffix(")"))
    {
        let pattern = if let Some(cmd) = inner.strip_suffix(":*") {
            format!("{} *", cmd)
        } else {
            inner.to_string()
        };
        return Some(ParsedPattern {
            category: PatternCategory::Shell,
            decision,
            pattern,
            domain: None,
            raw: raw.to_string(),
        });
    }

    if let Some(inner) = raw
        .strip_prefix("Edit(")
        .and_then(|s| s.strip_suffix(")"))
    {
        return Some(ParsedPattern {
            category: PatternCategory::Edit,
            decision,
            pattern: inner.to_string(),
            domain: None,
            raw: raw.to_string(),
        });
    }

    if let Some(inner) = raw
        .strip_prefix("Read(")
        .and_then(|s| s.strip_suffix(")"))
    {
        return Some(ParsedPattern {
            category: PatternCategory::Read,
            decision,
            pattern: inner.to_string(),
            domain: None,
            raw: raw.to_string(),
        });
    }

    if let Some(inner) = raw
        .strip_prefix("WebFetch(domain:")
        .and_then(|s| s.strip_suffix(")"))
    {
        return Some(ParsedPattern {
            category: PatternCategory::WebFetch,
            decision,
            pattern: inner.to_string(),
            domain: Some(inner.to_string()),
            raw: raw.to_string(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bash_prefix_wildcard() {
        let p = parse_pattern("Bash(git:*)", Decision::Allow).unwrap();
        assert_eq!(p.category, PatternCategory::Shell);
        assert_eq!(p.decision, Decision::Allow);
        assert_eq!(p.pattern, "git *");
        assert!(p.is_shell_prefix_wildcard());
        assert!(!p.is_shell_embedded_wildcard());
        assert_eq!(p.shell_base_command(), Some("git".to_string()));
    }

    #[test]
    fn parse_bash_exact() {
        let p = parse_pattern("Bash(npm test)", Decision::Allow).unwrap();
        assert_eq!(p.category, PatternCategory::Shell);
        assert_eq!(p.pattern, "npm test");
        assert!(!p.is_shell_prefix_wildcard());
        assert!(!p.is_shell_embedded_wildcard());
        assert_eq!(p.shell_base_command(), Some("npm test".to_string()));
    }

    #[test]
    fn parse_bash_embedded_wildcard() {
        let p = parse_pattern("Bash(git push --force *)", Decision::Deny).unwrap();
        assert_eq!(p.category, PatternCategory::Shell);
        assert!(p.is_shell_embedded_wildcard());
        assert_eq!(p.shell_base_command(), Some("git".to_string()));
    }

    #[test]
    fn parse_websearch() {
        let p = parse_pattern("WebSearch", Decision::Allow).unwrap();
        assert_eq!(p.category, PatternCategory::WebSearch);
    }

    #[test]
    fn parse_webfetch() {
        let p = parse_pattern("WebFetch(domain:example.com)", Decision::Allow).unwrap();
        assert_eq!(p.category, PatternCategory::WebFetch);
        assert_eq!(p.domain, Some("example.com".to_string()));
        assert_eq!(p.pattern, "example.com");
    }

    #[test]
    fn parse_edit() {
        let p = parse_pattern("Edit(**/.env)", Decision::Deny).unwrap();
        assert_eq!(p.category, PatternCategory::Edit);
        assert_eq!(p.pattern, "**/.env");
    }

    #[test]
    fn parse_read() {
        let p = parse_pattern("Read(/etc/passwd)", Decision::Deny).unwrap();
        assert_eq!(p.category, PatternCategory::Read);
        assert_eq!(p.pattern, "/etc/passwd");
    }

    #[test]
    fn parse_unrecognized_returns_none() {
        assert!(parse_pattern("SomethingUnknown(foo)", Decision::Allow).is_none());
    }

    #[test]
    fn template_permissions_all_patterns() {
        let mut tp = TemplatePermissions::default();
        tp.allow.push(parse_pattern("Bash(git:*)", Decision::Allow).unwrap());
        tp.deny.push(parse_pattern("Edit(**/.env)", Decision::Deny).unwrap());
        assert_eq!(tp.all_patterns().count(), 2);
        assert!(tp.has_any_deny());
    }
}
