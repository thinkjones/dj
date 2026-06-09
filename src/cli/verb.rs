#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Run,
    Doctor,
    Version,
    List,
    Info,
}

impl Verb {
    pub fn from_token(tok: &str) -> Option<Verb> {
        match tok {
            "run" => Some(Verb::Run),
            "doctor" => Some(Verb::Doctor),
            "version" => Some(Verb::Version),
            "list" => Some(Verb::List),
            "info" => Some(Verb::Info),
            _ => None,
        }
    }

    pub fn mutates(self) -> bool {
        matches!(self, Verb::Run)
    }
}

/// Resolve the effective verb from an explicit verb token (if any) and whether a
/// scope flag was supplied. Rules from the spec:
///   - explicit verb wins
///   - else a scope flag implies Run
///   - else Info (safe default)
pub fn resolve_verb(explicit: Option<Verb>, has_scope: bool) -> Verb {
    match (explicit, has_scope) {
        (Some(v), _) => v,
        (None, true) => Verb::Run,
        (None, false) => Verb::Info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_verbs() {
        assert_eq!(Verb::from_token("run"), Some(Verb::Run));
        assert_eq!(Verb::from_token("info"), Some(Verb::Info));
        assert_eq!(Verb::from_token("core"), None);
    }

    #[test]
    fn bare_name_defaults_to_info() {
        assert_eq!(resolve_verb(None, false), Verb::Info);
    }

    #[test]
    fn scope_flag_implies_run() {
        assert_eq!(resolve_verb(None, true), Verb::Run);
    }

    #[test]
    fn explicit_verb_wins_over_scope() {
        assert_eq!(resolve_verb(Some(Verb::Doctor), true), Verb::Doctor);
    }
}
