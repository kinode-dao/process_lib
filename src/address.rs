pub use crate::{Address, PackageId, ProcessId};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Address is defined in the wit bindings, but constructors and methods here.
/// An `Address` is a combination of an Uqbar node ID and a [`ProcessId`]. It is
/// used in the Request/Response pattern to indicate which process on a given node
/// in the Uqbar Network to direct the message to. The formatting structure for
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
    /// Attempt to parse an `Address` from a string. The formatting structure for
    /// an Address is `node@process_name:package_name:publisher_node`.
    ///
    /// TODO: clarify if `@` can be present in process name / package name / publisher name
    ///
    /// TODO: ensure `:` cannot sneak into first segment
    pub fn from_str(input: &str) -> Result<Self, AddressParseError> {
        // split string on colons into 4 segments,
        // first one with @, next 3 with :
        let mut name_rest = input.split('@');
        let node = name_rest
            .next()
            .ok_or(AddressParseError::MissingField)?
            .to_string();
        let mut segments = name_rest
            .next()
            .ok_or(AddressParseError::MissingNodeId)?
            .split(':');
        let process_name = segments
            .next()
            .ok_or(AddressParseError::MissingField)?
            .to_string();
        let package_name = segments
            .next()
            .ok_or(AddressParseError::MissingField)?
            .to_string();
        let publisher_node = segments
            .next()
            .ok_or(AddressParseError::MissingField)?
            .to_string();
        if segments.next().is_some() {
            return Err(AddressParseError::TooManyColons);
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
    /// not guaranteed to be a valid ID in the Uqbar name system, or be connected
    /// to an Uqbar identity at all.
    pub fn publisher(&self) -> &str {
        &self.process.publisher_node
    }
    /// Read the package_id (package + publisher) from an `Address`.
    pub fn package_id(&self) -> PackageId {
        PackageId::new(self.package(), self.publisher())
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
        Address::from_str(&s).map_err(serde::de::Error::custom)
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

impl<T> From<(&str, T)> for Address
where
    T: Into<ProcessId>,
{
    fn from(input: (&str, T)) -> Self {
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
    TooManyColons,
    MissingNodeId,
    MissingField,
}

impl std::fmt::Display for AddressParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AddressParseError::TooManyColons => "Too many colons in ProcessId string",
                AddressParseError::MissingNodeId => "Node ID missing",
                AddressParseError::MissingField => "Missing field in ProcessId string",
            }
        )
    }
}

impl std::error::Error for AddressParseError {
    fn description(&self) -> &str {
        match self {
            AddressParseError::TooManyColons => "Too many colons in ProcessId string",
            AddressParseError::MissingNodeId => "Node ID missing",
            AddressParseError::MissingField => "Missing field in ProcessId string",
        }
    }
}
