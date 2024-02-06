pub use crate::{Address, Capability};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Capability is defined in the wit bindings, but constructors and methods here.
/// A `Capability` is a combination of an Address and a set of Params (a serialized
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
}

impl std::str::FromStr for Capability {
    type Err = CapabilityParseError;

    fn from_str(s: &str) -> Result<Self, CapabilityParseError> {
        let end_of_issuer_index = s
            .find('(')
            .ok_or_else(|| CapabilityParseError::MissingParenthesis)?;
        let start_of_params_index = end_of_issuer_index + 1;
        let params_end_index = s
            .rfind(')')
            .ok_or_else(|| CapabilityParseError::MissingParenthesis)?;

        if params_end_index <= start_of_params_index {
            return Err(CapabilityParseError::MissingParenthesis);
        }

        let issuer_str = &s[..end_of_issuer_index];
        let params_str = &s[start_of_params_index..params_end_index];

        let Ok(issuer) = issuer_str.parse::<Address>() else {
            return Err(CapabilityParseError::InvalidAddress);
        };

        Ok(Capability {
            issuer,
            params: params_str.to_string(),
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
        s.parse().map_err(serde::de::Error::custom)
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
        write!(f, "{}({})", self.issuer, self.params)
    }
}

/// Error type for parsing an `Address` from a string.
#[derive(Debug)]
pub enum CapabilityParseError {
    MissingParenthesis,
    MissingParams,
    InvalidAddress,
}

impl std::fmt::Display for CapabilityParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CapabilityParseError::MissingParenthesis => "Missing parenthesis around Capability",
                CapabilityParseError::MissingParams => "Missing params in Capability string",
                CapabilityParseError::InvalidAddress => "Invalid address in Capability string",
            }
        )
    }
}

impl std::error::Error for CapabilityParseError {
    fn description(&self) -> &str {
        match self {
            CapabilityParseError::MissingParenthesis => "Missing parenthesis around Capability",
            CapabilityParseError::MissingParams => "Missing params in Capability string",
            CapabilityParseError::InvalidAddress => "Invalid address in Capability string",
        }
    }
}
