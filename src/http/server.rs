use crate::vfs::{FileType, VfsAction, VfsRequest, VfsResponse};
use crate::{
    get_blob, last_blob, LazyLoadBlob as KiBlob, Message, Request as KiRequest,
    Response as KiResponse,
};
pub use http::StatusCode;
use http::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// [`crate::Request`] received from the `http-server:distro:sys` service as a
/// result of either an HTTP or WebSocket binding, created via [`HttpServerAction`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HttpServerRequest {
    Http(IncomingHttpRequest),
    /// Processes will receive this kind of request when a client connects to them.
    /// If a process does not want this websocket open, they should issue a [`crate::Request`]
    /// containing a [`HttpServerAction::WebSocketClose`] message and this channel ID.
    WebSocketOpen {
        path: String,
        channel_id: u32,
    },
    /// Processes can both SEND and RECEIVE this kind of [`crate::Request`]
    /// (send as [`HttpServerAction::WebSocketPush`]).
    /// When received, will contain the message bytes as [`crate::LazyLoadBlob`].
    WebSocketPush {
        channel_id: u32,
        message_type: WsMessageType,
    },
    /// Receiving will indicate that the client closed the socket. Can be sent to close
    /// from the server-side, as [`type@HttpServerAction::WebSocketClose`].
    WebSocketClose(u32),
}

impl HttpServerRequest {
    /// Parse a byte slice into an [`HttpServerRequest`].
    pub fn from_bytes(bytes: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(bytes)
    }

    /// Filter the general-purpose [`HttpServerRequest`], which contains HTTP requests
    /// and WebSocket messages, into just the HTTP request. Consumes the original request
    /// and returns `None` if the request was WebSocket-related.
    pub fn request(self) -> Option<IncomingHttpRequest> {
        match self {
            HttpServerRequest::Http(req) => Some(req),
            _ => None,
        }
    }
}

/// An HTTP request routed to a process as a result of a binding.
///
/// BODY is stored in the lazy_load_blob, as bytes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncomingHttpRequest {
    /// will parse to [`std::net::SocketAddr`]
    source_socket_addr: Option<String>,
    /// will parse to [`http::Method`]
    method: String,
    /// will parse to [`url::Url`]
    url: String,
    /// the matching path that was bound
    bound_path: String,
    /// will parse to [`http::HeaderMap`]
    headers: HashMap<String, String>,
    url_params: HashMap<String, String>,
    query_params: HashMap<String, String>,
}

impl IncomingHttpRequest {
    pub fn url(&self) -> Result<url::Url, url::ParseError> {
        url::Url::parse(&self.url)
    }

    pub fn method(&self) -> Result<http::Method, http::method::InvalidMethod> {
        http::Method::from_bytes(self.method.as_bytes())
    }

    pub fn source_socket_addr(&self) -> Result<std::net::SocketAddr, std::net::AddrParseError> {
        match &self.source_socket_addr {
            Some(addr) => addr.parse(),
            None => "".parse(),
        }
    }

    /// Returns the path that was originally bound, with an optional prefix stripped.
    /// The prefix would normally be the process ID as a &str, but it could be anything.
    pub fn bound_path(&self, process_id_to_strip: Option<&str>) -> &str {
        match process_id_to_strip {
            Some(process_id) => self
                .bound_path
                .strip_prefix(&format!("/{}", process_id))
                .unwrap_or(&self.bound_path),
            None => &self.bound_path,
        }
    }

    pub fn path(&self) -> Result<String, url::ParseError> {
        let url = url::Url::parse(&self.url)?;
        // skip the first path segment, which is the process ID.
        let Some(path) = url.path_segments() else {
            return Err(url::ParseError::InvalidDomainCharacter);
        };
        let path = path.skip(1).collect::<Vec<&str>>().join("/");
        Ok(format!("/{}", path))
    }

    pub fn headers(&self) -> HeaderMap {
        let mut header_map = HeaderMap::new();
        for (key, value) in self.headers.iter() {
            let key_bytes = key.as_bytes();
            let Ok(key_name) = HeaderName::from_bytes(key_bytes) else {
                continue;
            };
            let Ok(value_header) = HeaderValue::from_str(&value) else {
                continue;
            };
            header_map.insert(key_name, value_header);
        }
        header_map
    }

    pub fn url_params(&self) -> &HashMap<String, String> {
        &self.url_params
    }

    pub fn query_params(&self) -> &HashMap<String, String> {
        &self.query_params
    }
}

/// The possible message types for [`HttpServerRequest::WebSocketPush`].
/// Ping and Pong are limited to 125 bytes by the WebSockets protocol.
/// Text will be sent as a Text frame, with the [`crate::LazyLoadBlob`] bytes
/// being the UTF-8 encoding of the string. Binary will be sent as a
/// Binary frame containing the unmodified [`crate::LazyLoadBlob`] bytes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum WsMessageType {
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}

/// [`crate::Request`] type sent to `http-server:distro:sys` in order to configure it.
///
/// If a [`crate::Response`] is expected, all actions will return a [`crate::Response`]
/// with the shape `Result<(), HttpServerActionError>` serialized to JSON.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HttpServerAction {
    /// Bind expects a [`crate::LazyLoadBlob`] if and only if `cache` is TRUE.
    /// The [`crate::LazyLoadBlob`] should be the static file to serve at this path.
    Bind {
        path: String,
        /// Set whether the HTTP request needs a valid login cookie, AKA, whether
        /// the user needs to be logged in to access this path.
        authenticated: bool,
        /// Set whether [`crate::Request`]s can be fielded from anywhere, or only the loopback address.
        local_only: bool,
        /// Set whether to bind the [`crate::LazyLoadBlob`] statically to this path. That is, take the
        /// [`crate::LazyLoadBlob`] bytes and serve them as the response to any request to this path.
        cache: bool,
    },
    /// SecureBind expects a [`crate::LazyLoadBlob`] if and only if `cache` is TRUE. The [`crate::LazyLoadBlob`] should
    /// be the static file to serve at this path.
    ///
    /// SecureBind is the same as Bind, except that it forces requests to be made from
    /// the unique subdomain of the process that bound the path. These requests are
    /// *always* authenticated, and *never* local_only. The purpose of SecureBind is to
    /// serve elements of an app frontend or API in an exclusive manner, such that other
    /// apps installed on this node cannot access them. Since the subdomain is unique, it
    /// will require the user to be logged in separately to the general domain authentication.
    SecureBind {
        path: String,
        /// Set whether to bind the [`crate::LazyLoadBlob`] statically to this path. That is, take the
        /// [`crate::LazyLoadBlob`] bytes and serve them as the response to any request to this path.
        cache: bool,
    },
    /// Unbind a previously-bound HTTP path
    Unbind { path: String },
    /// Bind a path to receive incoming WebSocket connections.
    /// Doesn't need a cache since does not serve assets.
    WebSocketBind {
        path: String,
        authenticated: bool,
        extension: bool,
    },
    /// SecureBind is the same as Bind, except that it forces new connections to be made
    /// from the unique subdomain of the process that bound the path. These are *always*
    /// authenticated. Since the subdomain is unique, it will require the user to be
    /// logged in separately to the general domain authentication.
    WebSocketSecureBind { path: String, extension: bool },
    /// Unbind a previously-bound WebSocket path
    WebSocketUnbind { path: String },
    /// When sent, expects a [`crate::LazyLoadBlob`] containing the WebSocket message bytes to send.
    WebSocketPush {
        channel_id: u32,
        message_type: WsMessageType,
    },
    /// When sent, expects a [`crate::LazyLoadBlob`] containing the WebSocket message bytes to send.
    /// Modifies the [`crate::LazyLoadBlob`] by placing into [`HttpServerAction::WebSocketExtPushData`]` with id taken from
    /// this [`KernelMessage`]` and `kinode_message_type` set to `desired_reply_type`.
    WebSocketExtPushOutgoing {
        channel_id: u32,
        message_type: WsMessageType,
        desired_reply_type: MessageType,
    },
    /// For communicating with the ext.
    /// Kinode's http-server sends this to the ext after receiving [`HttpServerAction::WebSocketExtPushOutgoing`].
    /// Upon receiving reply with this type from ext, http-server parses, setting:
    /// * id as given,
    /// * message type as given ([`crate::Request`] or [`crate::Response`]),
    /// * body as [`HttpServerRequest::WebSocketPush`],
    /// * [`crate::LazyLoadBlob`] as given.
    WebSocketExtPushData {
        id: u64,
        kinode_message_type: MessageType,
        blob: Vec<u8>,
    },
    /// Sending will close a socket the process controls.
    WebSocketClose(u32),
}

/// HTTP Response type that can be shared over Wasm boundary to apps.
/// Respond to [`IncomingHttpRequest`] with this type.
///
/// BODY is stored in the [`crate::LazyLoadBlob`] as bytes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
}

impl HttpResponse {
    pub fn new<T>(status: T) -> Self
    where
        T: Into<u16>,
    {
        Self {
            status: status.into(),
            headers: HashMap::new(),
        }
    }

    pub fn set_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    pub fn header<T, U>(mut self, key: T, value: U) -> Self
    where
        T: Into<String>,
        U: Into<String>,
    {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn set_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }
}

/// Part of the [`crate::Response`] type issued by http-server
#[derive(Clone, Debug, Error, Serialize, Deserialize)]
pub enum HttpServerError {
    #[error("request could not be deserialized to valid HttpServerRequest")]
    MalformedRequest,
    #[error("action expected blob")]
    NoBlob,
    #[error("path binding error: invalid source process")]
    InvalidSourceProcess,
    #[error("WebSocket error: ping/pong message too long")]
    WsPingPongTooLong,
    #[error("WebSocket error: channel not found")]
    WsChannelNotFound,
    /// Not actually issued by `http-server:distro:sys`, just this library
    #[error("timeout")]
    Timeout,
    /// Not actually issued by `http-server:distro:sys`, just this library
    #[error("unexpected response from http-server")]
    UnexpectedResponse,
}

/// Whether the [`HttpServerAction::WebSocketPush`] is [`crate::Request`] or [`crate::Response`].
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum MessageType {
    Request,
    Response,
}

/// A representation of the HTTP server as configured by your process.
#[derive(Clone, Debug)]
pub struct HttpServer {
    http_paths: HashMap<String, HttpBindingConfig>,
    ws_paths: HashMap<String, WsBindingConfig>,
    /// A mapping of WebSocket paths to the channels that are open on them.
    ws_channels: HashMap<String, HashSet<u32>>,
    /// The timeout given for `http-server:distro:sys` to respond to a configuration request.
    pub timeout: u64,
}

/// Configuration for a HTTP binding.
///
/// `authenticated` is set to true by default and means that the HTTP server will
/// require a valid login cookie to access this path.
///
/// `local_only` is set to false by default and means that the HTTP server will
/// only accept requests from the loopback address.
///
/// If `static_content` is set, the HTTP server will serve the static content at the
/// given path. Otherwise, the HTTP server will forward requests on this path to the
/// calling process.
///
/// If `secure_subdomain` is set, the HTTP server will serve requests on this path
/// from the unique subdomain of the process that bound the path. These requests are
/// *always* authenticated, and *never* local_only. The purpose of SecureBind is to
/// serve elements of an app frontend or API in an exclusive manner, such that other
/// apps installed on this node cannot access them. Since the subdomain is unique, it
/// will require the user to be logged in separately to the general domain authentication.
#[derive(Clone, Debug)]
pub struct HttpBindingConfig {
    authenticated: bool,
    local_only: bool,
    secure_subdomain: bool,
    static_content: Option<KiBlob>,
}

impl HttpBindingConfig {
    /// Create a new HttpBindingConfig with default values.
    ///
    /// Authenticated, not local only, not a secure subdomain, no static content.
    pub fn default() -> Self {
        Self {
            authenticated: true,
            local_only: false,
            secure_subdomain: false,
            static_content: None,
        }
    }

    /// Create a new HttpBindingConfig with the given values.
    pub fn new(
        authenticated: bool,
        local_only: bool,
        secure_subdomain: bool,
        static_content: Option<KiBlob>,
    ) -> Self {
        Self {
            authenticated,
            local_only,
            secure_subdomain,
            static_content,
        }
    }

    /// Set whether the HTTP server will require a valid login cookie to access this path.
    pub fn authenticated(mut self, authenticated: bool) -> Self {
        self.authenticated = authenticated;
        self
    }

    /// Set whether the HTTP server will only accept requests from the loopback address.
    pub fn local_only(mut self, local_only: bool) -> Self {
        self.local_only = local_only;
        self
    }

    /// Set whether the HTTP server will serve requests on this path from the unique
    /// subdomain of the process that bound the path. These requests are *always*
    /// authenticated, and *never* local_only. The purpose of SecureBind is to
    /// serve elements of an app frontend or API in an exclusive manner, such that other
    /// apps installed on this node cannot access them. Since the subdomain is unique, it
    /// will require the user to be logged in separately to the general domain authentication.
    pub fn secure_subdomain(mut self, secure_subdomain: bool) -> Self {
        self.secure_subdomain = secure_subdomain;
        self
    }

    /// Set the static content to serve at this path. If set, the HTTP server will
    /// not forward requests on this path to the process, and will instead serve the
    /// static content directly and only in response to  GET requests.
    pub fn static_content(mut self, static_content: Option<KiBlob>) -> Self {
        self.static_content = static_content;
        self
    }
}

/// Configuration for a WebSocket binding.
///
/// `authenticated` is set to true by default and means that the WebSocket server will
/// require a valid login cookie to access this path.
///
/// `extension` is set to false by default and means that the WebSocket will
/// not use the WebSocket extension protocol to connect with a runtime extension.
#[derive(Clone, Copy, Debug)]
pub struct WsBindingConfig {
    authenticated: bool,
    secure_subdomain: bool,
    extension: bool,
}

impl WsBindingConfig {
    /// Create a new WsBindingConfig with default values.
    ///
    /// Authenticated, not a secure subdomain, not an extension.
    pub fn default() -> Self {
        Self {
            authenticated: true,
            secure_subdomain: false,
            extension: false,
        }
    }

    /// Create a new WsBindingConfig with the given values.
    pub fn new(authenticated: bool, secure_subdomain: bool, extension: bool) -> Self {
        Self {
            authenticated,
            secure_subdomain,
            extension,
        }
    }

    /// Set whether the WebSocket server will require a valid login cookie to access this path.
    pub fn authenticated(mut self, authenticated: bool) -> Self {
        self.authenticated = authenticated;
        self
    }

    /// Set whether the WebSocket server will be bound on a secure subdomain.
    pub fn secure_subdomain(mut self, secure_subdomain: bool) -> Self {
        self.secure_subdomain = secure_subdomain;
        self
    }

    /// Set whether the WebSocket server will be used for a runtime extension.
    pub fn extension(mut self, extension: bool) -> Self {
        self.extension = extension;
        self
    }
}

impl HttpServer {
    /// Create a new HttpServer with the given timeout.
    pub fn new(timeout: u64) -> Self {
        Self {
            http_paths: HashMap::new(),
            ws_paths: HashMap::new(),
            ws_channels: HashMap::new(),
            timeout,
        }
    }

    /// Register a new path with the HTTP server configured using [`HttpBindingConfig`].
    pub fn bind_http_path<T>(
        &mut self,
        path: T,
        config: HttpBindingConfig,
    ) -> Result<(), HttpServerError>
    where
        T: Into<String>,
    {
        let path: String = path.into();
        let cache = config.static_content.is_some();
        let req = KiRequest::to(("our", "http-server", "distro", "sys")).body(
            serde_json::to_vec(&if config.secure_subdomain {
                HttpServerAction::SecureBind {
                    path: path.clone(),
                    cache,
                }
            } else {
                HttpServerAction::Bind {
                    path: path.clone(),
                    authenticated: config.authenticated,
                    local_only: config.local_only,
                    cache,
                }
            })
            .unwrap(),
        );
        let res = match config.static_content.clone() {
            Some(static_content) => req
                .blob(static_content)
                .send_and_await_response(self.timeout),
            None => req.send_and_await_response(self.timeout),
        };
        let Ok(Message::Response { body, .. }) = res.unwrap() else {
            return Err(HttpServerError::Timeout);
        };
        let Ok(resp) = serde_json::from_slice::<Result<(), HttpServerError>>(&body) else {
            return Err(HttpServerError::UnexpectedResponse);
        };
        if resp.is_ok() {
            self.http_paths.insert(path, config);
        }
        resp
    }

    /// Register a new path with the HTTP server configured using [`WsBindingConfig`].
    pub fn bind_ws_path<T>(
        &mut self,
        path: T,
        config: WsBindingConfig,
    ) -> Result<(), HttpServerError>
    where
        T: Into<String>,
    {
        let path: String = path.into();
        let res = KiRequest::to(("our", "http-server", "distro", "sys"))
            .body(if config.secure_subdomain {
                serde_json::to_vec(&HttpServerAction::WebSocketSecureBind {
                    path: path.clone(),
                    extension: config.extension,
                })
                .unwrap()
            } else {
                serde_json::to_vec(&HttpServerAction::WebSocketBind {
                    path: path.clone(),
                    authenticated: config.authenticated,
                    extension: config.extension,
                })
                .unwrap()
            })
            .send_and_await_response(self.timeout);
        let Ok(Message::Response { body, .. }) = res.unwrap() else {
            return Err(HttpServerError::Timeout);
        };
        let Ok(resp) = serde_json::from_slice::<Result<(), HttpServerError>>(&body) else {
            return Err(HttpServerError::UnexpectedResponse);
        };
        if resp.is_ok() {
            self.ws_paths.insert(path, config);
        }
        resp
    }

    /// Register a new path with the HTTP server, and serve a static file from it.
    /// The server will respond to GET requests on this path with the given file.
    pub fn bind_http_static_path<T>(
        &mut self,
        path: T,
        authenticated: bool,
        local_only: bool,
        content_type: Option<String>,
        content: Vec<u8>,
    ) -> Result<(), HttpServerError>
    where
        T: Into<String>,
    {
        let path: String = path.into();
        let res = KiRequest::to(("our", "http-server", "distro", "sys"))
            .body(
                serde_json::to_vec(&HttpServerAction::Bind {
                    path: path.clone(),
                    authenticated,
                    local_only,
                    cache: true,
                })
                .unwrap(),
            )
            .blob(crate::kinode::process::standard::LazyLoadBlob {
                mime: content_type.clone(),
                bytes: content.clone(),
            })
            .send_and_await_response(self.timeout)
            .unwrap();
        let Ok(Message::Response { body, .. }) = res else {
            return Err(HttpServerError::Timeout);
        };
        let Ok(resp) = serde_json::from_slice::<Result<(), HttpServerError>>(&body) else {
            return Err(HttpServerError::UnexpectedResponse);
        };
        if resp.is_ok() {
            self.http_paths.insert(
                path,
                HttpBindingConfig {
                    authenticated,
                    local_only,
                    secure_subdomain: false,
                    static_content: Some(KiBlob {
                        mime: content_type,
                        bytes: content,
                    }),
                },
            );
        }
        resp
    }

    /// Register a new path with the HTTP server. This will cause the HTTP server to
    /// forward any requests on this path to the calling process.
    ///
    /// Instead of binding at just a path, this function tells the HTTP server to
    /// generate a *subdomain* with our package ID (with non-ascii-alphanumeric
    /// characters converted to `-`, although will not be needed if package ID is
    /// a genuine kimap entry) and bind at that subdomain.
    pub fn secure_bind_http_path<T>(&mut self, path: T) -> Result<(), HttpServerError>
    where
        T: Into<String>,
    {
        let path: String = path.into();
        let res = KiRequest::to(("our", "http-server", "distro", "sys"))
            .body(
                serde_json::to_vec(&HttpServerAction::SecureBind {
                    path: path.clone(),
                    cache: false,
                })
                .unwrap(),
            )
            .send_and_await_response(self.timeout)
            .unwrap();
        let Ok(Message::Response { body, .. }) = res else {
            return Err(HttpServerError::Timeout);
        };
        let Ok(resp) = serde_json::from_slice::<Result<(), HttpServerError>>(&body) else {
            return Err(HttpServerError::UnexpectedResponse);
        };
        if resp.is_ok() {
            self.http_paths.insert(
                path,
                HttpBindingConfig {
                    authenticated: true,
                    local_only: false,
                    secure_subdomain: true,
                    static_content: None,
                },
            );
        }
        resp
    }

    /// Register a new WebSocket path with the HTTP server. Any client connections
    /// made on this path will be forwarded to this process.
    ///
    /// Instead of binding at just a path, this function tells the HTTP server to
    /// generate a *subdomain* with our package ID (with non-ascii-alphanumeric
    /// characters converted to `-`, although will not be needed if package ID is
    /// a genuine kimap entry) and bind at that subdomain.
    pub fn secure_bind_ws_path<T>(&mut self, path: T) -> Result<(), HttpServerError>
    where
        T: Into<String>,
    {
        let path: String = path.into();
        let res = KiRequest::to(("our", "http-server", "distro", "sys"))
            .body(
                serde_json::to_vec(&HttpServerAction::WebSocketSecureBind {
                    path: path.clone(),
                    extension: false,
                })
                .unwrap(),
            )
            .send_and_await_response(self.timeout);
        let Ok(Message::Response { body, .. }) = res.unwrap() else {
            return Err(HttpServerError::Timeout);
        };
        let Ok(resp) = serde_json::from_slice::<Result<(), HttpServerError>>(&body) else {
            return Err(HttpServerError::UnexpectedResponse);
        };
        if resp.is_ok() {
            self.ws_paths.insert(
                path,
                WsBindingConfig {
                    authenticated: true,
                    secure_subdomain: true,
                    extension: false,
                },
            );
        }
        resp
    }

    /// Modify a previously-bound HTTP path.
    pub fn modify_http_path<T>(
        &mut self,
        path: &str,
        config: HttpBindingConfig,
    ) -> Result<(), HttpServerError>
    where
        T: Into<String>,
    {
        let entry = self
            .http_paths
            .get_mut(path)
            .ok_or(HttpServerError::MalformedRequest)?;
        let res = KiRequest::to(("our", "http-server", "distro", "sys"))
            .body(
                serde_json::to_vec(&HttpServerAction::Bind {
                    path: path.to_string(),
                    authenticated: config.authenticated,
                    local_only: config.local_only,
                    cache: true,
                })
                .unwrap(),
            )
            .send_and_await_response(self.timeout)
            .unwrap();
        let Ok(Message::Response { body, .. }) = res else {
            return Err(HttpServerError::Timeout);
        };
        let Ok(resp) = serde_json::from_slice::<Result<(), HttpServerError>>(&body) else {
            return Err(HttpServerError::UnexpectedResponse);
        };
        if resp.is_ok() {
            entry.authenticated = config.authenticated;
            entry.local_only = config.local_only;
            entry.secure_subdomain = config.secure_subdomain;
            entry.static_content = config.static_content;
        }
        resp
    }

    /// Modify a previously-bound WS path
    pub fn modify_ws_path(
        &mut self,
        path: &str,
        config: WsBindingConfig,
    ) -> Result<(), HttpServerError> {
        let entry = self
            .ws_paths
            .get_mut(path)
            .ok_or(HttpServerError::MalformedRequest)?;
        let res = KiRequest::to(("our", "http-server", "distro", "sys"))
            .body(if entry.secure_subdomain {
                serde_json::to_vec(&HttpServerAction::WebSocketSecureBind {
                    path: path.to_string(),
                    extension: config.extension,
                })
                .unwrap()
            } else {
                serde_json::to_vec(&HttpServerAction::WebSocketBind {
                    path: path.to_string(),
                    authenticated: config.authenticated,
                    extension: config.extension,
                })
                .unwrap()
            })
            .send_and_await_response(self.timeout)
            .unwrap();
        let Ok(Message::Response { body, .. }) = res else {
            return Err(HttpServerError::Timeout);
        };
        let Ok(resp) = serde_json::from_slice::<Result<(), HttpServerError>>(&body) else {
            return Err(HttpServerError::UnexpectedResponse);
        };
        if resp.is_ok() {
            entry.authenticated = config.authenticated;
            entry.secure_subdomain = config.secure_subdomain;
            entry.extension = config.extension;
        }
        resp
    }

    /// Unbind a previously-bound HTTP path.
    pub fn unbind_http_path<T>(&mut self, path: T) -> Result<(), HttpServerError>
    where
        T: Into<String>,
    {
        let path: String = path.into();
        let res = KiRequest::to(("our", "http-server", "distro", "sys"))
            .body(serde_json::to_vec(&HttpServerAction::Unbind { path: path.clone() }).unwrap())
            .send_and_await_response(self.timeout)
            .unwrap();
        let Ok(Message::Response { body, .. }) = res else {
            return Err(HttpServerError::Timeout);
        };
        let Ok(resp) = serde_json::from_slice::<Result<(), HttpServerError>>(&body) else {
            return Err(HttpServerError::UnexpectedResponse);
        };
        if resp.is_ok() {
            self.http_paths.remove(&path);
        }
        resp
    }

    /// Unbind a previously-bound WebSocket path.
    pub fn unbind_ws_path<T>(&mut self, path: T) -> Result<(), HttpServerError>
    where
        T: Into<String>,
    {
        let path: String = path.into();
        let res = KiRequest::to(("our", "http-server", "distro", "sys"))
            .body(
                serde_json::to_vec(&HttpServerAction::WebSocketUnbind { path: path.clone() })
                    .unwrap(),
            )
            .send_and_await_response(self.timeout)
            .unwrap();
        let Ok(Message::Response { body, .. }) = res else {
            return Err(HttpServerError::Timeout);
        };
        let Ok(resp) = serde_json::from_slice::<Result<(), HttpServerError>>(&body) else {
            return Err(HttpServerError::UnexpectedResponse);
        };
        if resp.is_ok() {
            self.ws_paths.remove(&path);
        }
        resp
    }

    /// Serve a file from the given directory within our package drive at the given paths.
    ///
    /// The directory is relative to the `pkg` folder within this package's drive.
    ///
    /// The config `static_content` field will be ignored in favor of the file content.
    /// An error will be returned if the file does not exist.
    pub fn serve_file(
        &mut self,
        file_path: &str,
        paths: Vec<&str>,
        config: HttpBindingConfig,
    ) -> Result<(), HttpServerError> {
        let our = crate::our();
        let _res = KiRequest::to(("our", "vfs", "distro", "sys"))
            .body(
                serde_json::to_vec(&VfsRequest {
                    path: format!(
                        "/{}/pkg/{}",
                        our.package_id(),
                        file_path.trim_start_matches('/')
                    ),
                    action: VfsAction::Read,
                })
                .map_err(|_| HttpServerError::MalformedRequest)?,
            )
            .send_and_await_response(self.timeout)
            .unwrap();

        let Some(mut blob) = get_blob() else {
            return Err(HttpServerError::NoBlob);
        };

        let content_type = get_mime_type(&file_path);
        blob.mime = Some(content_type);

        for path in paths {
            self.bind_http_path(path, config.clone().static_content(Some(blob.clone())))?;
        }

        Ok(())
    }

    /// Serve a file from the given absolute directory.
    ///
    /// The config `static_content` field will be ignored in favor of the file content.
    /// An error will be returned if the file does not exist.
    pub fn serve_file_raw_path(
        &mut self,
        file_path: &str,
        paths: Vec<&str>,
        config: HttpBindingConfig,
    ) -> Result<(), HttpServerError> {
        let _res = KiRequest::to(("our", "vfs", "distro", "sys"))
            .body(
                serde_json::to_vec(&VfsRequest {
                    path: file_path.to_string(),
                    action: VfsAction::Read,
                })
                .map_err(|_| HttpServerError::MalformedRequest)?,
            )
            .send_and_await_response(self.timeout)
            .unwrap();

        let Some(mut blob) = get_blob() else {
            return Err(HttpServerError::NoBlob);
        };

        let content_type = get_mime_type(&file_path);
        blob.mime = Some(content_type);

        for path in paths {
            self.bind_http_path(path, config.clone().static_content(Some(blob.clone())))?;
        }

        Ok(())
    }

    /// Serve static files from a given directory by binding all of them
    /// in http-server to their filesystem path.
    ///
    /// The directory is relative to the `pkg` folder within this package's drive.
    ///
    /// The config `static_content` field will be ignored in favor of the files' contents.
    /// An error will be returned if the file does not exist.
    pub fn serve_ui(
        &mut self,
        directory: &str,
        roots: Vec<&str>,
        config: HttpBindingConfig,
    ) -> Result<(), HttpServerError> {
        let our = crate::our();
        let initial_path = format!("{}/pkg/{}", our.package_id(), directory);

        let mut queue = std::collections::VecDeque::new();
        queue.push_back(initial_path.clone());

        while let Some(path) = queue.pop_front() {
            let Ok(directory_response) = KiRequest::to(("our", "vfs", "distro", "sys"))
                .body(
                    serde_json::to_vec(&VfsRequest {
                        path,
                        action: VfsAction::ReadDir,
                    })
                    .unwrap(),
                )
                .send_and_await_response(self.timeout)
                .unwrap()
            else {
                return Err(HttpServerError::MalformedRequest);
            };

            let directory_body = serde_json::from_slice::<VfsResponse>(directory_response.body())
                .map_err(|_e| HttpServerError::UnexpectedResponse)?;

            // determine if it's a file or a directory and handle appropriately
            let VfsResponse::ReadDir(directory_info) = directory_body else {
                return Err(HttpServerError::UnexpectedResponse);
            };

            for entry in directory_info {
                match entry.file_type {
                    FileType::Directory => {
                        // push the directory onto the queue
                        queue.push_back(entry.path);
                    }
                    FileType::File => {
                        // if it's a file, serve it statically at its path
                        // if it's `index.html`, serve additionally as the root
                        if entry.path.ends_with("index.html") {
                            for root in &roots {
                                self.serve_file_raw_path(
                                    &entry.path,
                                    vec![root, &entry.path.replace(&initial_path, "")],
                                    config.clone(),
                                )?;
                            }
                        } else {
                            self.serve_file_raw_path(
                                &entry.path,
                                vec![&entry.path.replace(&initial_path, "")],
                                config.clone(),
                            )?;
                        }
                    }
                    _ => {
                        // ignore symlinks and other
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle a WebSocket open event from the HTTP server.
    pub fn handle_websocket_open(&mut self, path: &str, channel_id: u32) {
        self.ws_channels
            .entry(path.to_string())
            .or_insert(HashSet::new())
            .insert(channel_id);
    }

    /// Handle a WebSocket close event from the HTTP server.
    pub fn handle_websocket_close(&mut self, channel_id: u32) {
        self.ws_channels.iter_mut().for_each(|(_, channels)| {
            channels.remove(&channel_id);
        });
    }

    pub fn parse_request(&self, body: &[u8]) -> Result<HttpServerRequest, HttpServerError> {
        let request = serde_json::from_slice::<HttpServerRequest>(body)
            .map_err(|_| HttpServerError::MalformedRequest)?;
        Ok(request)
    }

    /// Handle an incoming request from the HTTP server.
    pub fn handle_request(
        &mut self,
        server_request: HttpServerRequest,
        mut http_handler: impl FnMut(IncomingHttpRequest) -> (HttpResponse, Option<KiBlob>),
        mut ws_handler: impl FnMut(u32, WsMessageType, KiBlob),
    ) {
        match server_request {
            HttpServerRequest::Http(http_request) => {
                let (response, blob) = http_handler(http_request);
                let response = KiResponse::new().body(serde_json::to_vec(&response).unwrap());
                if let Some(blob) = blob {
                    response.blob(blob).send().unwrap();
                } else {
                    response.send().unwrap();
                }
            }
            HttpServerRequest::WebSocketPush {
                channel_id,
                message_type,
            } => ws_handler(channel_id, message_type, last_blob().unwrap_or_default()),
            HttpServerRequest::WebSocketOpen { path, channel_id } => {
                self.handle_websocket_open(&path, channel_id);
            }
            HttpServerRequest::WebSocketClose(channel_id) => {
                self.handle_websocket_close(channel_id);
            }
        }
    }

    /// Push a WebSocket message to all channels on a given path.
    pub fn ws_push_all_channels(&self, path: &str, message_type: WsMessageType, blob: KiBlob) {
        ws_push_all_channels(&self.ws_channels, path, message_type, blob);
    }

    pub fn get_ws_channels(&self) -> HashMap<String, HashSet<u32>> {
        self.ws_channels.clone()
    }

    /// Register multiple paths with the HTTP server using the same configuration.
    /// The security setting is determined by the `secure_subdomain` field in `HttpBindingConfig`.
    /// All paths must be bound successfully, or none will be bound. If any path
    /// fails to bind, all previously bound paths will be unbound before returning
    /// the error.
    pub fn bind_multiple_http_paths<T: Into<String>>(
        &mut self,
        paths: Vec<T>,
        config: HttpBindingConfig,
    ) -> Result<(), HttpServerError> {
        let mut bound_paths = Vec::new();

        for path in paths {
            let path_str = path.into();
            let result = match config.secure_subdomain {
                true => self.secure_bind_http_path(path_str.clone()),
                false => self.bind_http_path(path_str.clone(), config.clone()),
            };

            match result {
                // If binding succeeds, add the path to the list of bound paths
                Ok(_) => bound_paths.push(path_str),
                // If binding fails, unbind all previously bound paths
                Err(e) => {
                    for bound_path in bound_paths {
                        let _ = self.unbind_http_path(&bound_path);
                    }
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}

/// Send an HTTP response to an incoming HTTP request ([`HttpServerRequest::Http`]).
pub fn send_response(status: StatusCode, headers: Option<HashMap<String, String>>, body: Vec<u8>) {
    KiResponse::new()
        .body(
            serde_json::to_vec(&HttpResponse {
                status: status.as_u16(),
                headers: headers.unwrap_or_default(),
            })
            .unwrap(),
        )
        .blob_bytes(body)
        .send()
        .unwrap()
}

/// Send a WebSocket push message on an open WebSocket channel.
pub fn send_ws_push(channel_id: u32, message_type: WsMessageType, blob: KiBlob) {
    KiRequest::to(("our", "http-server", "distro", "sys"))
        .body(
            serde_json::to_vec(&HttpServerRequest::WebSocketPush {
                channel_id,
                message_type,
            })
            .unwrap(),
        )
        .blob(blob)
        .send()
        .unwrap()
}

pub fn ws_push_all_channels(
    ws_channels: &HashMap<String, HashSet<u32>>,
    path: &str,
    message_type: WsMessageType,
    blob: KiBlob,
) {
    if let Some(channels) = ws_channels.get(path) {
        channels.iter().for_each(|channel_id| {
            send_ws_push(*channel_id, message_type, blob.clone());
        });
    }
}

/// Guess the MIME type of a file from its extension.
pub fn get_mime_type(filename: &str) -> String {
    let file_path = std::path::Path::new(filename);

    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("octet-stream");

    mime_guess::from_ext(extension)
        .first_or_octet_stream()
        .to_string()
}
