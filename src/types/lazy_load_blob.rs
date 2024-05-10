pub use crate::LazyLoadBlob;

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
