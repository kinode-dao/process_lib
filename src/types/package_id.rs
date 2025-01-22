pub use crate::PackageId;
use crate::ProcessIdParseError;
use serde::{Deserialize, Serialize};

/// `PackageId` is defined in the wit bindings, but constructors and methods
/// are defined here. A `PackageId` contains a package name and a publisher node ID.
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

impl Serialize for PackageId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PackageId", 2)?;
        state.serialize_field("package_name", &self.package_name)?;
        state.serialize_field("publisher_node", &self.publisher_node)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for PackageId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use serde::de::{Deserialize, Visitor};
        enum Field {
            PackageName,
            PublisherNode,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("`package_name` or `publisher_node`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "package_name" => Ok(Field::PackageName),
                            "publisher_node" => Ok(Field::PublisherNode),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                const FIELDS: &'static [&'static str] = &["package_name", "publisher_node"];
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct PackageIdVisitor;

        impl<'de> Visitor<'de> for PackageIdVisitor {
            type Value = PackageId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct PackageId")
            }

            fn visit_map<V>(self, mut map: V) -> Result<PackageId, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut package_name = None;
                let mut publisher_node = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::PackageName => {
                            if package_name.is_some() {
                                return Err(serde::de::Error::duplicate_field("package_name"));
                            }
                            package_name = Some(map.next_value()?);
                        }
                        Field::PublisherNode => {
                            if publisher_node.is_some() {
                                return Err(serde::de::Error::duplicate_field("publisher_node"));
                            }
                            publisher_node = Some(map.next_value()?);
                        }
                    }
                }
                let package_name =
                    package_name.ok_or_else(|| serde::de::Error::missing_field("package_name"))?;
                let publisher_node = publisher_node
                    .ok_or_else(|| serde::de::Error::missing_field("publisher_node"))?;
                Ok(PackageId {
                    package_name,
                    publisher_node,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["package_name", "publisher_node"];
        deserializer.deserialize_struct("PackageId", FIELDS, PackageIdVisitor)
    }
}

impl std::str::FromStr for PackageId {
    type Err = ProcessIdParseError;
    /// Attempts to parse a `PackageId` from a string. The string must match the pattern
    /// of two segments containing only lowercase letters, numbers and hyphens, separated by a colon.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let re = regex::Regex::new(r"^[a-z0-9-]+:[a-z0-9-.]+$").unwrap();
        if !re.is_match(input) {
            return Err(ProcessIdParseError::InvalidCharacter);
        }

        let segments: Vec<&str> = input.split(':').collect();
        Ok(PackageId {
            package_name: segments[0].to_string(),
            publisher_node: segments[1].to_string(),
        })
    }
}

impl std::hash::Hash for PackageId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.package_name.hash(state);
        self.publisher_node.hash(state);
    }
}

impl std::fmt::Display for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.package_name, self.publisher_node)
    }
}

impl From<(&str, &str)> for PackageId {
    fn from(input: (&str, &str)) -> Self {
        PackageId::new(input.0, input.1)
    }
}

impl std::cmp::Eq for PackageId {}

impl PartialEq for PackageId {
    fn eq(&self, other: &Self) -> bool {
        self.package_name == other.package_name && self.publisher_node == other.publisher_node
    }
}
