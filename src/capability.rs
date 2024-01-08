pub use crate::{Address, Capability, ProcessId};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Capability is defined in the wit bindings, but constructors and methods here.
/// A `Capability` is a combination of an Nectar Address and a set of Params (a serialized
/// json string). Capabilities are attached to messages to either share that capability
/// with the receiving process, or to prove that a process has authority to perform a
/// certain action.
impl Capability {
    /// Create a new `Capability`. Takes a node ID and a process ID.
    pub fn new<T, U>(address: T, params: U) -> Capability
    where
        T: Into<Address>,
        U: Into<String>,
    {
        Capability {
            issuer: address.into(),
            params: params.into(),
        }
    }
    /// Read the node ID from a `Capability`.
    pub fn issuer(&self) -> &Address {
        &self.issuer
    }
    /// Read the params from a `Capability`.
    pub fn params(&self) -> &str {
        &self.params
    }
    /// Attempt to parse a `Capability` from a string. The formatting structure for
    /// a Capability is `issuer^params`.
    /// TODO not tested
    pub fn from_str(input: &str) -> Result<Self, CapabilityParseError> {
        // split string on colons into 4 segments,
        // first one with @, next 3 with :
        let mut name_rest = input.split('@');
        let node = name_rest
            .next()
            .ok_or(CapabilityParseError::MissingField)?
            .to_string();
        let mut param_segments = name_rest
            .next()
            .ok_or(CapabilityParseError::MissingNodeId)?
            .split('^');
        let mut segments = param_segments
            .next()
            .ok_or(CapabilityParseError::MissingNodeId)?
            .split(':');
        let process_name = segments
            .next()
            .ok_or(CapabilityParseError::MissingField)?
            .to_string();
        let package_name = segments
            .next()
            .ok_or(CapabilityParseError::MissingField)?
            .to_string();
        let publisher_node = segments
            .next()
            .ok_or(CapabilityParseError::MissingField)?
            .to_string();
        let params = param_segments
            .next()
            .ok_or(CapabilityParseError::MissingParams)?
            .to_string();
        if segments.next().is_some() {
            return Err(CapabilityParseError::TooManyColons);
        }
        Ok(Capability {
            issuer: Address {
                node,
                process: ProcessId {
                    process_name,
                    package_name,
                    publisher_node,
                },
            },
            params,
        })
    }
}

impl Serialize for Capability {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        format!("{}", self).serialize(serializer)
    }
}

impl<'a> Deserialize<'a> for Capability {
    fn deserialize<D>(deserializer: D) -> Result<Capability, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        Capability::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Hash for Capability {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.issuer.hash(state);
        self.params.hash(state);
    }
}

impl Eq for Capability {}

impl PartialEq for Capability {
    fn eq(&self, other: &Self) -> bool {
        self.issuer == other.issuer && self.params == other.params
    }
}

impl From<&Capability> for Capability {
    fn from(input: &Capability) -> Self {
        input.clone()
    }
}

impl<T> From<(T, &str)> for Capability
where
    T: Into<Address>,
{
    fn from(input: (T, &str)) -> Self {
        Capability::new(input.0, input.1)
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}^{}", self.issuer, self.params)
    }
}

/// Error type for parsing an `Address` from a string.
#[derive(Debug)]
pub enum CapabilityParseError {
    TooManyColons,
    MissingNodeId,
    MissingField,
    MissingParams,
}

impl std::fmt::Display for CapabilityParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CapabilityParseError::TooManyColons => "Too many colons in ProcessId string",
                CapabilityParseError::MissingNodeId => "Node ID missing",
                CapabilityParseError::MissingField => "Missing field in ProcessId string",
                CapabilityParseError::MissingParams => "Missing params in Capability string",
            }
        )
    }
}

impl std::error::Error for CapabilityParseError {
    fn description(&self) -> &str {
        match self {
            CapabilityParseError::TooManyColons => "Too many colons in ProcessId string",
            CapabilityParseError::MissingNodeId => "Node ID missing",
            CapabilityParseError::MissingField => "Missing field in ProcessId string",
            CapabilityParseError::MissingParams => "Missing params in Capability string",
        }
    }
}
