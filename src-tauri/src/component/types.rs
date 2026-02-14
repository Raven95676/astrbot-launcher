use serde::Serialize;

/// Identifies a managed component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentId {
    Python312,
    Python310,
}

impl ComponentId {
    /// Directory name under `components/`.
    pub fn dir_name(self) -> &'static str {
        match self {
            Self::Python312 => "python312",
            Self::Python310 => "python310",
        }
    }

    /// Human-readable display name.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Python312 => "Python 3.12",
            Self::Python310 => "Python 3.10",
        }
    }

    /// Major version string used by the download system (e.g. "3.12").
    pub fn major_version(self) -> &'static str {
        match self {
            Self::Python312 => "3.12",
            Self::Python310 => "3.10",
        }
    }

    /// Parse a string id (e.g. from the frontend) into a `ComponentId`.
    pub fn from_str_id(s: &str) -> Option<Self> {
        match s {
            "python312" => Some(Self::Python312),
            "python310" => Some(Self::Python310),
            _ => None,
        }
    }

    /// All known component ids.
    pub fn all() -> &'static [Self] {
        &[Self::Python312, Self::Python310]
    }
}

/// Status of a single component, sent to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentStatus {
    pub id: String,
    pub installed: bool,
    pub display_name: String,
    pub description: String,
}

/// Snapshot of all component statuses.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentsSnapshot {
    pub components: Vec<ComponentStatus>,
}
