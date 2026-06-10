pub mod dispatch;
pub mod onboarding;
pub mod parse;
pub mod scope;
pub mod verb;
pub mod workflow;

/// What a name resolves to. A name may be both a plugin and a workflow (conflict).
#[derive(Debug, PartialEq, Eq)]
pub enum Resolved {
    Plugin,
    Workflow,
    Both,
    None,
}
