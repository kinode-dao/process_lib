pub use crate::{Address, Capability};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct};
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

impl Serialize for Capability {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut state = serializer.serialize_struct("Capability", 2)?;
        state.serialize_field("issuer", &self.issuer)?;
        state.serialize_field("params", &self.params)?;
        state.end()
    }
}

impl<'a> Deserialize<'a> for Capability {
    fn deserialize<D>(deserializer: D) -> Result<Capability, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        enum Field {
            Issuer,
            Params,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("`issuer` or `params`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "issuer" => Ok(Field::Issuer),
                            "params" => Ok(Field::Params),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct CapabilityVisitor;

        impl<'de> Visitor<'de> for CapabilityVisitor {
            type Value = Capability;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Capability")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Capability, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let issuer: Address = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let params: String = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(Capability::new(issuer, params))
            }

            fn visit_map<V>(self, mut map: V) -> Result<Capability, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut issuer: Option<Address> = None;
                let mut params: Option<String> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Issuer => {
                            if issuer.is_some() {
                                return Err(de::Error::duplicate_field("issuer"));
                            }
                            issuer = Some(map.next_value()?);
                        }
                        Field::Params => {
                            if params.is_some() {
                                return Err(de::Error::duplicate_field("params"));
                            }
                            params = Some(map.next_value()?);
                        }
                    }
                }
                let issuer: Address = issuer
                    .ok_or_else(|| de::Error::missing_field("issuer"))?
                    .into();
                let params: String = params
                    .ok_or_else(|| de::Error::missing_field("params"))?
                    .into();
                Ok(Capability::new(issuer, params))
            }
        }

        const FIELDS: &'static [&'static str] = &["issuer", "params"];
        deserializer.deserialize_struct("Capability", FIELDS, CapabilityVisitor)
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
