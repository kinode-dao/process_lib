pub use crate::{Address, ProcessId, Request};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Address is defined in `kinode.wit`, but constructors and methods here.
/// An Address is a combination of a node ID (string) and a [`ProcessId`]. It is
/// used in the [`Request`]/[`crate::Response`] pattern to indicate which process on a given node
/// in the network to direct the message to. The formatting structure for
/// an Address is `node@process_name:package_name:publisher_node`
impl Address {
    /// Create a new `Address`. Takes a node ID and a process ID.
    pub fn new<T, U>(node: T, process: U) -> Address
    where
        T: Into<String>,
        U: Into<ProcessId>,
    {
        Address {
            node: node.into(),
            process: process.into(),
        }
    }
    /// Read the node ID from an `Address`.
    pub fn node(&self) -> &str {
        &self.node
    }
    /// Read the process name from an `Address`.
    pub fn process(&self) -> &str {
        &self.process.process_name
    }
    /// Read the package name from an `Address`.
    pub fn package(&self) -> &str {
        &self.process.package_name
    }
    /// Read the publisher node ID from an `Address`. Note that `Address`
    /// segments are not parsed for validity, and a node ID stored here is
    /// not guaranteed to be a valid ID in the name system, or be connected
    /// to an identity at all.
    pub fn publisher(&self) -> &str {
        &self.process.publisher_node
    }
    /// Read the package_id (package + publisher) from an `Address`.
    pub fn package_id(&self) -> crate::PackageId {
        crate::PackageId::new(self.package(), self.publisher())
    }

    /// Send a [`Request`] to `Address`.
    pub fn send_request(&self) -> Request {
        Request::to(self)
    }
}

impl std::str::FromStr for Address {
    type Err = AddressParseError;
    /// Attempt to parse an `Address` from a string. The formatting structure for
    /// an Address is `node@process_name:package_name:publisher_node`.
    ///
    /// The string being parsed must contain exactly one `@` and three `:` characters.
    /// The `@` character separates the node ID from the rest of the address, and the
    /// `:` characters separate the process name, package name, and publisher node ID.
    fn from_str(input: &str) -> Result<Self, AddressParseError> {
        // split string on '@' and ensure there is exactly one '@'
        let parts: Vec<&str> = input.split('@').collect();
        if parts.len() < 2 {
            return Err(AddressParseError::MissingNodeId);
        } else if parts.len() > 2 {
            return Err(AddressParseError::TooManyAts);
        }
        let node = parts[0].to_string();
        if node.is_empty() {
            return Err(AddressParseError::MissingNodeId);
        }

        // split the rest on ':' and ensure there are exactly three ':'
        let segments: Vec<&str> = parts[1].split(':').collect();
        if segments.len() < 3 {
            return Err(AddressParseError::MissingField);
        } else if segments.len() > 3 {
            return Err(AddressParseError::TooManyColons);
        }
        let process_name = segments[0].to_string();
        if process_name.is_empty() {
            return Err(AddressParseError::MissingField);
        }
        let package_name = segments[1].to_string();
        if package_name.is_empty() {
            return Err(AddressParseError::MissingField);
        }
        let publisher_node = segments[2].to_string();
        if publisher_node.is_empty() {
            return Err(AddressParseError::MissingField);
        }

        Ok(Address {
            node,
            process: ProcessId {
                process_name,
                package_name,
                publisher_node,
            },
        })
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        format!("{}", self).serialize(serializer)
    }
}

impl<'a> Deserialize<'a> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Address, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Hash for Address {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node.hash(state);
        self.process.hash(state);
    }
}

impl Eq for Address {}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node && self.process == other.process
    }
}

impl From<&Address> for Address {
    fn from(input: &Address) -> Self {
        input.clone()
    }
}

impl<T, U, V, W> From<(T, U, V, W)> for Address
where
    T: Into<String>,
    U: Into<&'static str>,
    V: Into<&'static str>,
    W: Into<&'static str>,
{
    fn from(input: (T, U, V, W)) -> Self {
        Address::new(
            input.0.into(),
            (input.1.into(), input.2.into(), input.3.into()),
        )
    }
}

impl<T, U> From<(T, U)> for Address
where
    T: Into<String>,
    U: Into<ProcessId>,
{
    fn from(input: (T, U)) -> Self {
        Address::new(input.0, input.1)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.node, self.process)
    }
}

/// Error type for parsing an `Address` from a string.
#[derive(Debug)]
pub enum AddressParseError {
    TooManyAts,
    TooManyColons,
    MissingNodeId,
    MissingField,
}

impl std::fmt::Display for AddressParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::error::Error for AddressParseError {
    fn description(&self) -> &str {
        match self {
            AddressParseError::TooManyAts => "Too many '@' chars in ProcessId string",
            AddressParseError::TooManyColons => "Too many colons in ProcessId string",
            AddressParseError::MissingNodeId => "Node ID missing",
            AddressParseError::MissingField => "Missing field in ProcessId string",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_valid_address() {
        let input = "node123@process1:packageA:publisherB";
        let address: Address = input.parse().unwrap();
        assert_eq!(address.node(), "node123");
        assert_eq!(address.process(), "process1");
        assert_eq!(address.package(), "packageA");
        assert_eq!(address.publisher(), "publisherB");
    }

    #[test]
    fn test_missing_node_id() {
        let input = "@process1:packageA:publisherB";
        assert!(matches!(
            Address::from_str(input),
            Err(AddressParseError::MissingNodeId)
        ));
    }

    #[test]
    fn test_too_many_ats() {
        let input = "node123@process1@packageA:publisherB";
        assert!(matches!(
            Address::from_str(input),
            Err(AddressParseError::TooManyAts)
        ));
    }

    #[test]
    fn test_missing_field() {
        let input = "node123@process1:packageA";
        assert!(matches!(
            Address::from_str(input),
            Err(AddressParseError::MissingField)
        ));
    }

    #[test]
    fn test_too_many_colons() {
        let input = "node123@process1:packageA:publisherB:extra";
        assert!(matches!(
            Address::from_str(input),
            Err(AddressParseError::TooManyColons)
        ));
    }

    #[test]
    fn test_empty_input() {
        let input = "";
        assert!(matches!(
            Address::from_str(input),
            Err(AddressParseError::MissingNodeId)
        ));
    }

    #[test]
    fn test_display() {
        let input = "node123@process1:packageA:publisherB";
        let address: Address = input.parse().unwrap();
        assert_eq!(format!("{}", address), input);
    }
}
