#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Install,
    Status,
    Version,
    Info,
}

impl Verb {
    pub fn from_token(tok: &str) -> Option<Verb> {
        match tok {
            "install" => Some(Verb::Install),
            "status" => Some(Verb::Status),
            "version" => Some(Verb::Version),
            "info" => Some(Verb::Info),
            _ => None,
        }
    }

    pub fn mutates(self) -> bool {
        matches!(self, Verb::Install)
    }
}

/// Resolve the effective verb from an explicit verb token (if any).
/// With no explicit verb, defaults to Info.
pub fn resolve_verb(explicit: Option<Verb>) -> Verb {
    explicit.unwrap_or(Verb::Info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_verbs() {
        assert_eq!(Verb::from_token("install"), Some(Verb::Install));
        assert_eq!(Verb::from_token("status"), Some(Verb::Status));
        assert_eq!(Verb::from_token("info"), Some(Verb::Info));
        assert_eq!(Verb::from_token("core"), None);
    }

    #[test]
    fn bare_name_defaults_to_info() {
        assert_eq!(resolve_verb(None), Verb::Info);
    }

    #[test]
    fn explicit_verb_wins() {
        assert_eq!(resolve_verb(Some(Verb::Status)), Verb::Status);
    }
}
