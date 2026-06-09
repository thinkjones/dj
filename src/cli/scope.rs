use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    User,
    Folder(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeKind {
    User,
    Folder,
}

impl Scope {
    pub fn kind(&self) -> ScopeKind {
        match self {
            Scope::User => ScopeKind::User,
            Scope::Folder(_) => ScopeKind::Folder,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn folder_defaults_to_cwd_marker() {
        let s = Scope::Folder(PathBuf::new());
        assert_eq!(s.kind(), ScopeKind::Folder);
    }

    #[test]
    fn user_kind() {
        assert_eq!(Scope::User.kind(), ScopeKind::User);
    }
}
