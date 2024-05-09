use crate::*;

/// Request builder. Use [`Request::new()`] to start a request, then build it,
/// then call [`Request::send()`] on it to fire.
#[derive(Clone, Debug)]
pub struct Request {
    pub target: Option<Address>,
    pub inherit: bool,
    pub timeout: Option<u64>,
    pub body: Option<Vec<u8>>,
    pub metadata: Option<String>,
    pub blob: Option<LazyLoadBlob>,
    pub context: Option<Vec<u8>>,
    pub capabilities: Vec<Capability>,
}

#[allow(dead_code)]
impl Request {
    /// Start building a new `Request`. In order to successfully send, a
    /// `Request` must have at least a `target` and an `body`. Calling send
    /// on this before filling out these fields will result in an error.
    pub fn new() -> Self {
        Request {
            target: None,
            inherit: false,
            timeout: None,
            body: None,
            metadata: None,
            blob: None,
            context: None,
            capabilities: vec![],
        }
    }
    /// Start building a new Request with the Address of the target. In order
    /// to successfully send, you must still fill out at least the `body` field
    /// by calling `body()` or `try_body()` next.
    pub fn to<T>(target: T) -> Self
    where
        T: Into<Address>,
    {
        Request {
            target: Some(target.into()),
            inherit: false,
            timeout: None,
            body: None,
            metadata: None,
            blob: None,
            context: None,
            capabilities: vec![],
        }
    }
    /// Set the target [`Address`] that this request will go to.
    pub fn target<T>(mut self, target: T) -> Self
    where
        T: Into<Address>,
    {
        self.target = Some(target.into());
        self
    }
    /// Set whether this request will "inherit" the source / context / blob
    /// of the request that this process most recently received. The purpose
    /// of inheritance, in this setting, is twofold:
    ///
    /// One, setting inherit to `true` and not attaching a `LazyLoadBlob` will result
    /// in the previous request's blob being attached to this request. This
    /// is useful for optimizing performance of middleware and other chains of
    /// requests that can pass large quantities of data through multiple
    /// processes without repeatedly pushing it across the Wasm boundary.
    ///
    /// *Note that if the blob of this request is set separately, this flag
    /// will not override it.*
    ///
    /// Two, setting inherit to `true` and *not expecting a response* will lead
    /// to the previous request's sender receiving the potential response to
    /// *this* request. This will only happen if the previous request's sender
    /// was expecting a response. This behavior chains, such that many processes
    /// could handle inheriting requests while passing the ultimate response back
    /// to the very first requester.
    pub fn inherit(mut self, inherit: bool) -> Self {
        self.inherit = inherit;
        self
    }
    /// Set whether this request expects a response, and provide a timeout value
    /// (in seconds) within which that response must be received. The sender will
    /// receive an error message with this request stored within it if the
    /// timeout is triggered.
    pub fn expects_response(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
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
    pub fn try_body<T>(mut self, body: T) -> anyhow::Result<Self>
    where
        T: TryInto<Vec<u8>, Error = anyhow::Error>,
    {
        self.body = Some(body.try_into()?);
        Ok(self)
    }
    /// Set the metdata field for this request. Metadata is simply a [`String`].
    /// Metadata should usually be used for middleware and other message-passing
    /// situations that require the original IPC body and blob to be preserved.
    /// As such, metadata should not always be expected to reach the final destination
    /// of this request unless the full chain of behavior is known / controlled by
    /// the developer.
    pub fn metadata(mut self, metadata: &str) -> Self {
        self.metadata = Some(metadata.to_string());
        self
    }
    /// Set the blob of this request. A [`LazyLoadBlob`] holds bytes and an optional
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
    pub fn try_blob_bytes<T>(mut self, bytes: T) -> anyhow::Result<Self>
    where
        T: TryInto<Vec<u8>, Error = anyhow::Error>,
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
    /// Set the context field of the request. A request's context is just another byte
    /// vector. The developer should create a strategy for serializing and deserializing
    /// contexts.
    ///
    /// Contexts are useful when avoiding "callback hell". When a request is sent, any
    /// response or error (timeout, offline node) will be returned with this context.
    /// This allows you to chain multiple asynchronous requests with their responses
    /// without using complex logic to store information about outstanding requests.
    pub fn context<T>(mut self, context: T) -> Self
    where
        T: Into<Vec<u8>>,
    {
        self.context = Some(context.into());
        self
    }
    /// Attempt to set the context field of the request with a type that implements
    /// `TryInto<Vec<u8>>`. It's best to define a context type within your app,
    /// then implement TryFrom/TryInto for all context serialization/deserialization.
    pub fn try_context<T>(mut self, context: T) -> anyhow::Result<Self>
    where
        T: TryInto<Vec<u8>, Error = anyhow::Error>,
    {
        self.context = Some(context.try_into()?);
        Ok(self)
    }
    /// Attach capabilities to the next request
    pub fn capabilities(mut self, capabilities: Vec<Capability>) -> Self {
        self.capabilities = capabilities;
        self
    }
    /// Attach the capability to message this process to the next message.
    pub fn attach_messaging(mut self, our: &Address) {
        self.capabilities.extend(vec![Capability {
            issuer: our.clone(),
            params: "\"messaging\"".to_string(),
        }]);
    }
    /// Attempt to send the request. This will only fail if the `target` or `body`
    /// fields have not been set.
    pub fn send(self) -> anyhow::Result<()> {
        if let (Some(target), Some(body)) = (self.target, self.body) {
            crate::send_request(
                &target,
                &crate::kinode::process::standard::Request {
                    inherit: self.inherit,
                    expects_response: self.timeout,
                    body,
                    metadata: self.metadata,
                    capabilities: self.capabilities,
                },
                self.context.as_ref(),
                self.blob.as_ref(),
            );
            Ok(())
        } else {
            Err(anyhow::anyhow!("missing fields"))
        }
    }
    /// Attempt to send the request, then await its response or error (timeout, offline node).
    /// This will only fail if the `target` or `body` fields have not been set.
    pub fn send_and_await_response(
        self,
        timeout: u64,
    ) -> anyhow::Result<Result<Message, SendError>> {
        if let (Some(target), Some(body)) = (self.target, self.body) {
            match crate::send_and_await_response(
                &target,
                &crate::kinode::process::standard::Request {
                    inherit: self.inherit,
                    expects_response: Some(timeout),
                    body,
                    metadata: self.metadata,
                    capabilities: self.capabilities,
                },
                self.blob.as_ref(),
            ) {
                Ok((source, message)) => Ok(Ok(wit_message_to_message(source, message))),
                Err(send_err) => Ok(Err(SendError {
                    kind: match send_err.kind {
                        crate::kinode::process::standard::SendErrorKind::Offline => {
                            SendErrorKind::Offline
                        }
                        crate::kinode::process::standard::SendErrorKind::Timeout => {
                            SendErrorKind::Timeout
                        }
                    },
                    message: wit_message_to_message(
                        Address::new("our", ProcessId::new(Some("net"), "distro", "sys")),
                        send_err.message,
                    ),
                    lazy_load_blob: send_err.lazy_load_blob,
                    context: None,
                })),
            }
        } else {
            Err(anyhow::anyhow!("missing fields"))
        }
    }
}

impl Default for Request {
    fn default() -> Self {
        Request::new()
    }
}
