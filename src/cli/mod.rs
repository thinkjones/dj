pub mod dispatch;
pub mod onboarding;
pub mod parse;
pub mod scope;
pub mod verb;
pub mod workflow;

/// Names that may never be a plugin or workflow (validated at registry build).
pub const RESERVED: &[&str] = &[
    "run", "doctor", "version", "list", "info", "self", "catalog", "all",
];

/// What a name resolves to. A name may be both a plugin and a workflow (conflict).
#[derive(Debug, PartialEq, Eq)]
pub enum Resolved {
    Plugin,
    Workflow,
    Both,
    None,
}
