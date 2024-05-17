use crate::ProcessIdParseError;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

/// PackageId is like a ProcessId, but for a package. Only contains the name
/// of the package and the name of the publisher.
#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct PackageId {
    package_name: String,
    publisher_node: String,
}

impl PackageId {
    /// Create a new `PackageId`.
    pub fn new(package_name: &str, publisher_node: &str) -> Self {
        PackageId {
            package_name: package_name.into(),
            publisher_node: publisher_node.into(),
        }
    }
    /// Read the package name from a `PackageId`.
    pub fn package(&self) -> &str {
        &self.package_name
    }
    /// Read the publisher node ID from a `PackageId`. Note that `PackageId`
    /// segments are not parsed for validity, and a node ID stored here is
    /// not guaranteed to be a valid ID in the name system, or be connected
    /// to an identity at all.
    pub fn publisher(&self) -> &str {
        &self.publisher_node
    }
}

impl std::str::FromStr for PackageId {
    type Err = ProcessIdParseError;
    /// Attempt to parse a `PackageId` from a string. The string must
    /// contain exactly two segments, where segments are non-empty strings
    /// separated by a colon (`:`). The segments cannot themselves contain colons.
    ///
    /// Please note that while any string without colons will parse successfully
    /// to create a `PackageId`, not all strings without colons are actually
    /// valid usernames, which the `publisher_node` field of a `PackageId` will
    /// always in practice be.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let segments: Vec<&str> = input.split(':').collect();
        if segments.len() < 2 {
            return Err(ProcessIdParseError::MissingField);
        } else if segments.len() > 2 {
            return Err(ProcessIdParseError::TooManyColons);
        }
        let package_name = segments[0].to_string();
        if package_name.is_empty() {
            return Err(ProcessIdParseError::MissingField);
        }
        let publisher_node = segments[1].to_string();
        if publisher_node.is_empty() {
            return Err(ProcessIdParseError::MissingField);
        }

        Ok(PackageId {
            package_name,
            publisher_node,
        })
    }
}

impl std::fmt::Display for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.package_name, self.publisher_node)
    }
}
