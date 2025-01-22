pub use crate::ProcessId;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// `ProcessId` is defined in the wit bindings, but constructors and methods
/// are defined here. A `ProcessId` contains a process name, a package name,
/// and a publisher node ID.
impl ProcessId {
    /// Create a new `ProcessId`. If `process_name` is left as None, this will generate
    /// a random u64 number, convert to string, and store that as the name.
    pub fn new(process_name: Option<&str>, package_name: &str, publisher_node: &str) -> Self {
        ProcessId {
            process_name: process_name
                .unwrap_or(&rand::random::<u64>().to_string())
                .into(),
            package_name: package_name.into(),
            publisher_node: publisher_node.into(),
        }
    }
    /// Read the process name from a `ProcessId`.
    pub fn process(&self) -> &str {
        &self.process_name
    }
    /// Read the package name from a `ProcessId`.
    pub fn package(&self) -> &str {
        &self.package_name
    }
    /// Read the publisher node ID from a `ProcessId`. Note that `ProcessId`
    /// segments are not parsed for validity, and a node ID stored here is
    /// not guaranteed to be a valid ID in the name system, or be connected
    /// to an identity at all.
    pub fn publisher(&self) -> &str {
        &self.publisher_node
    }
}

impl std::str::FromStr for ProcessId {
    type Err = ProcessIdParseError;
    /// Attempts to parse a `ProcessId` from a string. The string must match the pattern
    /// of three segments containing only lowercase letters, numbers and hyphens, separated by colons.
    fn from_str(input: &str) -> Result<Self, ProcessIdParseError> {
        let re = regex::Regex::new(r"^[a-z0-9-]+:[a-z0-9-]+:[a-z0-9-.]+$").unwrap();
        if !re.is_match(input) {
            return Err(ProcessIdParseError::InvalidCharacter);
        }

        let segments: Vec<&str> = input.split(':').collect();
        Ok(ProcessId {
            process_name: segments[0].to_string(),
            package_name: segments[1].to_string(),
            publisher_node: segments[2].to_string(),
        })
    }
}

impl Serialize for ProcessId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        format!("{}", self).serialize(serializer)
    }
}

impl<'a> Deserialize<'a> for ProcessId {
    fn deserialize<D>(deserializer: D) -> Result<ProcessId, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Hash for ProcessId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.process_name.hash(state);
        self.package_name.hash(state);
        self.publisher_node.hash(state);
    }
}

impl Eq for ProcessId {}

impl From<(&str, &str, &str)> for ProcessId {
    fn from(input: (&str, &str, &str)) -> Self {
        ProcessId::new(Some(input.0), input.1, input.2)
    }
}

impl std::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.process_name, self.package_name, self.publisher_node
        )
    }
}

impl PartialEq for ProcessId {
    fn eq(&self, other: &Self) -> bool {
        self.process_name == other.process_name
            && self.package_name == other.package_name
            && self.publisher_node == other.publisher_node
    }
}

impl PartialEq<&str> for ProcessId {
    fn eq(&self, other: &&str) -> bool {
        &self.to_string() == other
    }
}

impl PartialEq<ProcessId> for &str {
    fn eq(&self, other: &ProcessId) -> bool {
        self == &other.to_string()
    }
}

#[derive(Debug)]
pub enum ProcessIdParseError {
    TooManyColons,
    MissingField,
    InvalidCharacter,
}

impl std::fmt::Display for ProcessIdParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ProcessIdParseError::TooManyColons => "Too many colons",
                ProcessIdParseError::MissingField => "Missing field",
                ProcessIdParseError::InvalidCharacter => "Invalid character",
            }
        )
    }
}

impl std::error::Error for ProcessIdParseError {
    fn description(&self) -> &str {
        match self {
            ProcessIdParseError::TooManyColons => "Too many colons",
            ProcessIdParseError::MissingField => "Missing field",
            ProcessIdParseError::InvalidCharacter => "Invalid character",
        }
    }
}
