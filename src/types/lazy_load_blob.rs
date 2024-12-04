pub use crate::LazyLoadBlob;

/// `LazyLoadBlob` is defined in the wit bindings, but constructors and methods here.
/// A `LazyLoadBlob` is a piece of data that is only optionally loaded into a process
/// (i.e. with `get_blob()`). `LazyLoadBlob` is useful for passing large data in a chain
/// of [`crate::Request`]s or [`crate::Response`]s where intermediate processes in the
/// chain don't need to access the data. In this way, Kinode saves time and compute
/// since the `LazyLoadBlob` is not sent back and forth across the Wasm boundary needlessly.
impl LazyLoadBlob {
    /// Create a new `LazyLoadBlob`. Takes a mime type and a byte vector.
    pub fn new<T, U>(mime: Option<T>, bytes: U) -> LazyLoadBlob
    where
        T: Into<String>,
        U: Into<Vec<u8>>,
    {
        LazyLoadBlob {
            mime: mime.map(|mime| mime.into()),
            bytes: bytes.into(),
        }
    }
    /// Read the mime type from a `LazyLoadBlob`.
    pub fn mime(&self) -> Option<&str> {
        self.mime.as_ref().map(|mime| mime.as_str())
    }
    /// Read the bytes from a `LazyLoadBlob`.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl std::default::Default for LazyLoadBlob {
    fn default() -> Self {
        LazyLoadBlob {
            mime: None,
            bytes: Vec::new(),
        }
    }
}

impl std::cmp::PartialEq for LazyLoadBlob {
    fn eq(&self, other: &Self) -> bool {
        self.mime == other.mime && self.bytes == other.bytes
    }
}
