use serde::{Deserialize, Serialize};

/// How serious a diagnostic is. `error` stops nothing by itself — a plugin
/// that must stop returns an `err` envelope instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warn,
    Error,
}

/// Where in the source a diagnostic points: an entry, and a line within it
/// when the entry has lines.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct At {
    pub entry: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// A plugin's one way to report: in-band, in its outputs, and as the `err`
/// payload of a failed call.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub level: Level,
    pub msg: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<At>,
}

impl Diagnostic {
    pub fn new(level: Level, msg: impl Into<String>) -> Self {
        Self {
            level,
            msg: msg.into(),
            at: None,
        }
    }

    pub fn info(msg: impl Into<String>) -> Self {
        Self::new(Level::Info, msg)
    }

    pub fn warn(msg: impl Into<String>) -> Self {
        Self::new(Level::Warn, msg)
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self::new(Level::Error, msg)
    }

    /// Point the diagnostic at an entry, and a line within it.
    pub fn at(mut self, entry: impl Into<String>, line: Option<u32>) -> Self {
        self.at = Some(At {
            entry: entry.into(),
            line,
        });
        self
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.at {
            Some(At {
                entry,
                line: Some(line),
            }) => write!(f, "{entry}:{line}: {}", self.msg),
            Some(At { entry, line: None }) => write!(f, "{entry}: {}", self.msg),
            None => f.write_str(&self.msg),
        }
    }
}

impl std::error::Error for Diagnostic {}
