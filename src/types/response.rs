use crate::{types::message::BuildError, Capability, LazyLoadBlob};

/// Response builder. Use [`Response::new()`] to start a response, then build it,
/// then call [`Response::send()`] on it to fire.
pub struct Response {
    inherit: bool,
    body: Option<Vec<u8>>,
    metadata: Option<String>,
    blob: Option<LazyLoadBlob>,
    capabilities: Vec<Capability>,
}

impl Response {
    /// Start building a new response. Attempting to send this response will
    /// not succeed until its `body` has been set with `body()` or `try_body()`.
    pub fn new() -> Self {
        Response {
            inherit: false,
            body: None,
            metadata: None,
            blob: None,
            capabilities: vec![],
        }
    }
    /// Set whether this response will "inherit" the blob of the request
    /// that this process most recently received. Unlike with requests, the
    /// inherit field of a response only deals with blob attachment, since
    /// responses don't themselves have to consider responses or contexts.
    ///
    /// *Note that if the blob is set for this response, this flag will not
    /// override it.*
    pub fn inherit(mut self, inherit: bool) -> Self {
        self.inherit = inherit;
        self
    }
    /// Set the IPC body (Inter-Process Communication) value for this message. This field
    /// is mandatory. An IPC body is simply a vector of bytes. Process developers are
    /// responsible for architecting the serialization/derserialization strategy
    /// for these bytes, but the simplest and most common strategy is just to use
    /// a JSON spec that gets stored in bytes as a UTF-8 string.
    ///
    /// If the serialization strategy is complex, it's best to define it as an impl
    /// of [`TryInto`] on your IPC body type, then use `try_body()` instead of this.
    pub fn body<T>(mut self, body: T) -> Self
    where
        T: Into<Vec<u8>>,
    {
        self.body = Some(body.into());
        self
    }
    /// Set the IPC body (Inter-Process Communication) value for this message, using a
    /// type that's got an implementation of [`TryInto`] for `Vec<u8>`. It's best
    /// to define an IPC body type within your app, then implement TryFrom/TryInto for
    /// all IPC body serialization/deserialization.
    pub fn try_body<T, E>(mut self, body: T) -> Result<Self, E>
    where
        T: TryInto<Vec<u8>, Error = E>,
        E: std::error::Error,
    {
        self.body = Some(body.try_into()?);
        Ok(self)
    }
    /// Set the metadata field for this response. Metadata is simply a [`String`].
    /// Metadata should usually be used for middleware and other message-passing
    /// situations that require the original IPC body and blob to be preserved.
    /// As such, metadata should not always be expected to reach the final destination
    /// of this response unless the full chain of behavior is known / controlled by
    /// the developer.
    pub fn metadata(mut self, metadata: &str) -> Self {
        self.metadata = Some(metadata.to_string());
        self
    }
    /// Set the blob of this response. A [`LazyLoadBlob`] holds bytes and an optional
    /// MIME type.
    ///
    /// The purpose of having a blob field distinct from the IPC body field is to enable
    /// performance optimizations in all sorts of situations. LazyLoadBlobs are only brought
    /// across the runtime<>Wasm boundary if the process calls `get_blob()`, and this
    /// saves lots of work in data-intensive pipelines.
    ///
    /// LazyLoadBlobs also provide a place for less-structured data, such that an IPC body type
    /// can be quickly locked in and upgraded within an app-protocol without breaking
    /// changes, while still allowing freedom to adjust the contents and shape of a
    /// blob. IPC body formats should be rigorously defined.
    pub fn blob(mut self, blob: LazyLoadBlob) -> Self {
        self.blob = Some(blob);
        self
    }
    /// Set the blob's MIME type. If a blob has not been set, it will be set here
    /// as an empty vector of bytes. If it has been set, the MIME type will be replaced
    /// or created.
    pub fn blob_mime(mut self, mime: &str) -> Self {
        if self.blob.is_none() {
            self.blob = Some(LazyLoadBlob {
                mime: Some(mime.to_string()),
                bytes: vec![],
            });
            self
        } else {
            self.blob = Some(LazyLoadBlob {
                mime: Some(mime.to_string()),
                bytes: self.blob.unwrap().bytes,
            });
            self
        }
    }
    /// Set the blob's bytes. If a blob has not been set, it will be set here with
    /// no MIME type. If it has been set, the bytes will be replaced with these bytes.
    pub fn blob_bytes<T>(mut self, bytes: T) -> Self
    where
        T: Into<Vec<u8>>,
    {
        if self.blob.is_none() {
            self.blob = Some(LazyLoadBlob {
                mime: None,
                bytes: bytes.into(),
            });
            self
        } else {
            self.blob = Some(LazyLoadBlob {
                mime: self.blob.unwrap().mime,
                bytes: bytes.into(),
            });
            self
        }
    }
    /// Set the blob's bytes with a type that implements `TryInto<Vec<u8>>`
    /// and may or may not successfully be set.
    pub fn try_blob_bytes<T, E>(mut self, bytes: T) -> Result<Self, E>
    where
        T: TryInto<Vec<u8>, Error = E>,
        E: std::error::Error,
    {
        if self.blob.is_none() {
            self.blob = Some(LazyLoadBlob {
                mime: None,
                bytes: bytes.try_into()?,
            });
            Ok(self)
        } else {
            self.blob = Some(LazyLoadBlob {
                mime: self.blob.unwrap().mime,
                bytes: bytes.try_into()?,
            });
            Ok(self)
        }
    }
    /// Add capabilities to this response. Capabilities are a way to pass
    pub fn capabilities(mut self, capabilities: Vec<Capability>) -> Self {
        self.capabilities = capabilities;
        self
    }
    /// Attempt to send the response. This will only fail if the IPC body field of
    /// the response has not yet been set using `body()` or `try_body()`.
    pub fn send(self) -> Result<(), BuildError> {
        if let Some(body) = self.body {
            crate::send_response(
                &crate::kinode::process::standard::Response {
                    inherit: self.inherit,
                    body,
                    metadata: self.metadata,
                    capabilities: self.capabilities,
                },
                self.blob.as_ref(),
            );
            Ok(())
        } else {
            Err(BuildError::NoBody)
        }
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}
