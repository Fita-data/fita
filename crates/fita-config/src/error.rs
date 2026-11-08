use std::fmt;
use std::path::PathBuf;

/// A problem found in a project file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// File the problem is in, relative to the project folder.
    pub path: PathBuf,
    /// What is wrong.
    pub message: String,
}

impl Issue {
    pub(crate) fn new(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

/// Error returned by [`crate::load`].
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// The folder has no `fita.yaml`.
    #[error("{0} is not a Fita project, fita.yaml is missing")]
    NotAProject(PathBuf),
    /// The project has errors. Every error found is listed.
    #[error("the project has {} error(s):\n{}", .0.len(), list(.0))]
    Invalid(Vec<Issue>),
}

fn list(issues: &[Issue]) -> String {
    let lines: Vec<String> = issues.iter().map(|issue| format!("  {issue}")).collect();
    lines.join("\n")
}
