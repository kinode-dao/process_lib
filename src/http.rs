use crate::kernel_types::MessageType;
use crate::vfs::{FileType, VfsAction, VfsRequest, VfsResponse};
use crate::{
    get_blob, Address, LazyLoadBlob as KiBlob, Message, Request as KiRequest,
    Response as KiResponse,
};
pub use http::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;

//
// these types are a copy of the types used in http module of runtime.
//

/// HTTP Request type that can be shared over WASM boundary to apps.
/// This is the one you receive from the `http_server:distro:sys` service.
#[derive(Debug, Serialize, Deserialize)]
pub enum HttpServerRequest {
    Http(IncomingHttpRequest),
    /// Processes will receive this kind of request when a client connects to them.
    /// If a process does not want this websocket open, they should issue a *request*
    /// containing a [`type@HttpServerAction::WebSocketClose`] message and this channel ID.
    WebSocketOpen {
        path: String,
        channel_id: u32,
    },
    /// Processes can both SEND and RECEIVE this kind of request
    /// (send as [`type@HttpServerAction::WebSocketPush`]).
    /// When received, will contain the message bytes as lazy_load_blob.
    WebSocketPush {
        channel_id: u32,
        message_type: WsMessageType,
    },
    /// Receiving will indicate that the client closed the socket. Can be sent to close
    /// from the server-side, as [`type@HttpServerAction::WebSocketClose`].
    WebSocketClose(u32),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IncomingHttpRequest {
    source_socket_addr: Option<String>, // will parse to SocketAddr
    method: String,                     // will parse to http::Method
    url: String,                        // will parse to url::Url
    headers: HashMap<String, String>,   // will parse to http::HeaderMap
    query_params: HashMap<String, String>,
    // BODY is stored in the lazy_load_blob, as bytes
}

/// HTTP Response type that can be shared over WASM boundary to apps.
/// Respond to [`IncomingHttpRequest`] with this type.
#[derive(Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    // BODY is stored in the lazy_load_blob, as bytes
}

/// Request type sent to `http_server:distro:sys` in order to configure it.
/// You can also send [`type@HttpServerAction::WebSocketPush`], which
/// allows you to push messages across an existing open WebSocket connection.
///
/// If a response is expected, all HttpServerActions will return a Response
/// with the shape Result<(), HttpServerActionError> serialized to JSON.
#[derive(Debug, Serialize, Deserialize)]
pub enum HttpServerAction {
    /// Bind expects a lazy_load_blob if and only if `cache` is TRUE. The lazy_load_blob should
    /// be the static file to serve at this path.
    Bind {
        path: String,
        /// Set whether the HTTP request needs a valid login cookie, AKA, whether
        /// the user needs to be logged in to access this path.
        authenticated: bool,
        /// Set whether requests can be fielded from anywhere, or only the loopback address.
        local_only: bool,
        /// Set whether to bind the lazy_load_blob statically to this path. That is, take the
        /// lazy_load_blob bytes and serve them as the response to any request to this path.
        cache: bool,
    },
    /// SecureBind expects a lazy_load_blob if and only if `cache` is TRUE. The lazy_load_blob should
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
        /// Set whether to bind the lazy_load_blob statically to this path. That is, take the
        /// lazy_load_blob bytes and serve them as the response to any request to this path.
        cache: bool,
    },
    /// Bind a path to receive incoming WebSocket connections.
    /// Doesn't need a cache since does not serve assets.
    WebSocketBind {
        path: String,
        authenticated: bool,
        encrypted: bool,
        extension: bool,
    },
    /// SecureBind is the same as Bind, except that it forces new connections to be made
    /// from the unique subdomain of the process that bound the path. These are *always*
    /// authenticated. Since the subdomain is unique, it will require the user to be
    /// logged in separately to the general domain authentication.
    WebSocketSecureBind {
        path: String,
        encrypted: bool,
        extension: bool,
    },
    /// Processes will RECEIVE this kind of request when a client connects to them.
    /// If a process does not want this websocket open, they should issue a *request*
    /// containing a [`type@HttpServerAction::WebSocketClose`] message and this channel ID.
    WebSocketOpen { path: String, channel_id: u32 },
    /// When sent, expects a lazy_load_blob containing the WebSocket message bytes to send.
    WebSocketPush {
        channel_id: u32,
        message_type: WsMessageType,
    },
    /// When sent, expects a `lazy_load_blob` containing the WebSocket message bytes to send.
    /// Modifies the `lazy_load_blob` by placing into `WebSocketExtPushData` with id taken from
    /// this `KernelMessage` and `kinode_message_type` set to `desired_reply_type`.
    WebSocketExtPushOutgoing {
        channel_id: u32,
        message_type: WsMessageType,
        desired_reply_type: MessageType,
    },
    /// For communicating with the ext.
    /// Kinode's http_server sends this to the ext after receiving `WebSocketExtPushOutgoing`.
    /// Upon receiving reply with this type from ext, http_server parses, setting:
    /// * id as given,
    /// * message type as given (Request or Response),
    /// * body as HttpServerRequest::WebSocketPush,
    /// * blob as given.
    WebSocketExtPushData {
        id: u64,
        kinode_message_type: MessageType,
        blob: Vec<u8>,
    },
    /// Sending will close a socket the process controls.
    WebSocketClose(u32),
}

/// The possible message types for WebSocketPush. Ping and Pong are limited to 125 bytes
/// by the WebSockets protocol. Text will be sent as a Text frame, with the lazy_load_blob bytes
/// being the UTF-8 encoding of the string. Binary will be sent as a Binary frame containing
/// the unmodified lazy_load_blob bytes.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum WsMessageType {
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}

/// Part of the Response type issued by http_server
#[derive(Error, Debug, Serialize, Deserialize)]
pub enum HttpServerError {
    #[error(
        "http_server: request could not be parsed to HttpServerAction: {}.",
        req
    )]
    BadRequest { req: String },
    #[error("http_server: action expected lazy_load_blob")]
    NoBlob,
    #[error("http_server: path binding error: {:?}", error)]
    PathBindError { error: String },
    #[error("http_server: WebSocket error: {:?}", error)]
    WebSocketPushError { error: String },
}

/// Structure sent from client websocket to this server upon opening a new connection.
/// After this is sent, depending on the `encrypted` flag, the channel will either be
/// open to send and receive plaintext messages or messages encrypted with a symmetric
/// key derived from the JWT.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsRegister {
    pub auth_token: String,
    pub target_process: String,
    pub encrypted: bool, // TODO symmetric key exchange here if true
}

/// Structure sent from this server to client websocket upon opening a new connection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsRegisterResponse {
    pub channel_id: u32,
    // TODO symmetric key exchange here
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub username: String,
    pub expiration: u64,
}

impl HttpServerRequest {
    /// Parse a byte slice into an HttpServerRequest.
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

impl IncomingHttpRequest {
    pub fn url(&self) -> anyhow::Result<url::Url> {
        url::Url::parse(&self.url).map_err(|e| anyhow::anyhow!("couldn't parse url: {:?}", e))
    }

    pub fn method(&self) -> anyhow::Result<http::Method> {
        http::Method::from_bytes(self.method.as_bytes())
            .map_err(|e| anyhow::anyhow!("couldn't parse method: {:?}", e))
    }

    pub fn source_socket_addr(&self) -> anyhow::Result<std::net::SocketAddr> {
        match &self.source_socket_addr {
            Some(addr) => addr
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid format for socket address: {}", addr)),
            None => Err(anyhow::anyhow!("No source socket address provided")),
        }
    }

    pub fn path(&self) -> anyhow::Result<String> {
        let url = url::Url::parse(&self.url)?;
        // skip the first path segment, which is the process ID.
        let path = url
            .path_segments()
            .ok_or(anyhow::anyhow!("url path missing process ID!"))?
            .skip(1)
            .collect::<Vec<&str>>()
            .join("/");
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

    pub fn query_params(&self) -> &HashMap<String, String> {
        &self.query_params
    }
}

/// Request type that can be shared over WASM boundary to apps.
/// This is the one you send to the `http_client:distro:sys` service.
#[derive(Debug, Serialize, Deserialize)]
pub enum HttpClientAction {
    Http(OutgoingHttpRequest),
    WebSocketOpen {
        url: String,
        headers: HashMap<String, String>,
        channel_id: u32,
    },
    WebSocketPush {
        channel_id: u32,
        message_type: WsMessageType,
    },
    WebSocketClose {
        channel_id: u32,
    },
}

/// HTTP Request type that can be shared over WASM boundary to apps.
/// This is the one you send to the `http_client:distro:sys` service.
#[derive(Debug, Serialize, Deserialize)]
pub struct OutgoingHttpRequest {
    pub method: String,          // must parse to http::Method
    pub version: Option<String>, // must parse to http::Version
    pub url: String,             // must parse to url::Url
    pub headers: HashMap<String, String>,
    // BODY is stored in the lazy_load_blob, as bytes
    // TIMEOUT is stored in the message expect_response
}

/// WebSocket Client Request type that can be shared over WASM boundary to apps.
/// This comes from an open websocket client connection in the `http_client:distro:sys` service.
#[derive(Debug, Serialize, Deserialize)]
pub enum HttpClientRequest {
    WebSocketPush {
        channel_id: u32,
        message_type: WsMessageType,
    },
    WebSocketClose {
        channel_id: u32,
    },
}

/// HTTP Client Response type that can be shared over WASM boundary to apps.
/// This is the one you receive from the `http_client:distro:sys` service.
#[derive(Debug, Serialize, Deserialize)]
pub enum HttpClientResponse {
    Http(HttpResponse),
    WebSocketAck,
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum HttpClientError {
    // HTTP errors, may also be applicable to OutgoingWebSocketClientRequest::Open
    #[error("http_client: request is not valid HttpClientRequest: {}.", req)]
    BadRequest { req: String },
    #[error("http_client: http method not supported: {}", method)]
    BadMethod { method: String },
    #[error("http_client: url could not be parsed: {}", url)]
    BadUrl { url: String },
    #[error("http_client: http version not supported: {}", version)]
    BadVersion { version: String },
    #[error("http_client: failed to execute request {}", error)]
    RequestFailed { error: String },

    // WebSocket errors
    #[error("websocket_client: failed to open connection {}", url)]
    WsOpenFailed { url: String },
    #[error("websocket_client: failed to send message {}", req)]
    WsPushFailed { req: String },
    #[error("websocket_client: failed to close connection {}", channel_id)]
    WsCloseFailed { channel_id: u32 },
}

/// Register a new path with the HTTP server. This will cause the HTTP server to
/// forward any requests on this path to the calling process.
pub fn bind_http_path<T>(
    path: T,
    authenticated: bool,
    local_only: bool,
) -> std::result::Result<(), HttpServerError>
where
    T: Into<String>,
{
    let res = KiRequest::to(("our", "http_server", "distro", "sys"))
        .body(
            serde_json::to_vec(&HttpServerAction::Bind {
                path: path.into(),
                authenticated,
                local_only,
                cache: false,
            })
            .unwrap(),
        )
        .send_and_await_response(5)
        .unwrap();
    let Ok(Message::Response { body, .. }) = res else {
        return Err(HttpServerError::PathBindError {
            error: "http_server timed out".to_string(),
        });
    };
    let Ok(resp) = serde_json::from_slice::<std::result::Result<(), HttpServerError>>(&body) else {
        return Err(HttpServerError::PathBindError {
            error: "http_server gave unexpected response".to_string(),
        });
    };
    resp
}

/// Register a new path with the HTTP server, and serve a static file from it.
/// The server will respond to GET requests on this path with the given file.
pub fn bind_http_static_path<T>(
    path: T,
    authenticated: bool,
    local_only: bool,
    content_type: Option<String>,
    content: Vec<u8>,
) -> std::result::Result<(), HttpServerError>
where
    T: Into<String>,
{
    let res = KiRequest::to(("our", "http_server", "distro", "sys"))
        .body(
            serde_json::to_vec(&HttpServerAction::Bind {
                path: path.into(),
                authenticated,
                local_only,
                cache: true,
            })
            .unwrap(),
        )
        .blob(crate::kinode::process::standard::LazyLoadBlob {
            mime: content_type,
            bytes: content,
        })
        .send_and_await_response(5)
        .unwrap();
    let Ok(Message::Response { body, .. }) = res else {
        return Err(HttpServerError::PathBindError {
            error: "http_server timed out".to_string(),
        });
    };
    let Ok(resp) = serde_json::from_slice::<std::result::Result<(), HttpServerError>>(&body) else {
        return Err(HttpServerError::PathBindError {
            error: "http_server gave unexpected response".to_string(),
        });
    };
    resp
}

/// Register a WebSockets path with the HTTP server. Your app must do this
/// in order to receive incoming WebSocket connections.
pub fn bind_ws_path<T>(
    path: T,
    authenticated: bool,
    encrypted: bool,
) -> std::result::Result<(), HttpServerError>
where
    T: Into<String>,
{
    let res = KiRequest::to(("our", "http_server", "distro", "sys"))
        .body(
            serde_json::to_vec(&HttpServerAction::WebSocketBind {
                path: path.into(),
                authenticated,
                encrypted,
                extension: false,
            })
            .unwrap(),
        )
        .send_and_await_response(5)
        .unwrap();
    let Ok(Message::Response { body, .. }) = res else {
        return Err(HttpServerError::PathBindError {
            error: "http_server timed out".to_string(),
        });
    };
    let Ok(resp) = serde_json::from_slice::<std::result::Result<(), HttpServerError>>(&body) else {
        return Err(HttpServerError::PathBindError {
            error: "http_server gave unexpected response".to_string(),
        });
    };
    resp
}

/// Register a WebSockets path with the HTTP server to send and
/// receive system messages from a runtime extension. Only use
/// this if you are writing a runtime extension.
pub fn bind_ext_path<T>(path: T) -> std::result::Result<(), HttpServerError>
where
    T: Into<String>,
{
    let res = KiRequest::to(("our", "http_server", "distro", "sys"))
        .body(
            serde_json::to_vec(&HttpServerAction::WebSocketBind {
                path: path.into(),
                authenticated: false,
                encrypted: false,
                extension: true,
            })
            .unwrap(),
        )
        .send_and_await_response(5)
        .unwrap();
    let Ok(Message::Response { body, .. }) = res else {
        return Err(HttpServerError::PathBindError {
            error: "http_server timed out".to_string(),
        });
    };
    let Ok(resp) = serde_json::from_slice::<std::result::Result<(), HttpServerError>>(&body) else {
        return Err(HttpServerError::PathBindError {
            error: "http_server gave unexpected response".to_string(),
        });
    };
    resp
}

/// Send an HTTP response to the incoming HTTP request.
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

/// Fire off an HTTP request. If a timeout is given, the response will
/// come in the main event loop, otherwise none will be given.
///
/// Note that the response type is [`type@HttpClientResponse`], which, if
/// it originated from this request, will be of the variant [`type@HttpClientResponse::Http`].
/// It will need to be parsed and the body of the response will be stored in the LazyLoadBlob.
pub fn send_request(
    method: Method,
    url: url::Url,
    headers: Option<HashMap<String, String>>,
    timeout: Option<u64>,
    body: Vec<u8>,
) {
    let req = KiRequest::to(("our", "http_client", "distro", "sys"))
        .body(
            serde_json::to_vec(&HttpClientAction::Http(OutgoingHttpRequest {
                method: method.to_string(),
                version: None,
                url: url.to_string(),
                headers: headers.unwrap_or_default(),
            }))
            .unwrap(),
        )
        .blob_bytes(body);
    if let Some(timeout) = timeout {
        req.expects_response(timeout).send().unwrap()
    } else {
        req.send().unwrap()
    }
}

/// Make an HTTP request using http_client and await its response.
///
/// Returns [`Response`] from the `http` crate if successful, with the body type as bytes.
pub fn send_request_await_response(
    method: Method,
    url: url::Url,
    headers: Option<HashMap<String, String>>,
    timeout: u64,
    body: Vec<u8>,
) -> std::result::Result<Response<Vec<u8>>, HttpClientError> {
    let res = KiRequest::to(("our", "http_client", "distro", "sys"))
        .body(
            serde_json::to_vec(&HttpClientAction::Http(OutgoingHttpRequest {
                method: method.to_string(),
                version: None,
                url: url.to_string(),
                headers: headers.unwrap_or_default(),
            }))
            .map_err(|e| HttpClientError::BadRequest {
                req: format!("{e:?}"),
            })?,
        )
        .blob_bytes(body)
        .send_and_await_response(timeout)
        .unwrap();
    let Ok(Message::Response { body, .. }) = res else {
        return Err(HttpClientError::RequestFailed {
            error: "http_client timed out".to_string(),
        });
    };
    let resp = match serde_json::from_slice::<
        std::result::Result<HttpClientResponse, HttpClientError>,
    >(&body)
    {
        Ok(Ok(HttpClientResponse::Http(resp))) => resp,
        Ok(Ok(HttpClientResponse::WebSocketAck)) => {
            return Err(HttpClientError::RequestFailed {
                error: "http_client gave unexpected response".to_string(),
            })
        }
        Ok(Err(e)) => return Err(e),
        Err(e) => {
            return Err(HttpClientError::RequestFailed {
                error: format!("http_client gave invalid response: {e:?}"),
            })
        }
    };
    let mut http_response = http::Response::builder()
        .status(http::StatusCode::from_u16(resp.status).unwrap_or_default());
    let headers = http_response.headers_mut().unwrap();
    for (key, value) in &resp.headers {
        let Ok(key) = http::header::HeaderName::from_str(key) else {
            return Err(HttpClientError::RequestFailed {
                error: format!("http_client gave invalid header key: {key}"),
            });
        };
        let Ok(value) = http::header::HeaderValue::from_str(value) else {
            return Err(HttpClientError::RequestFailed {
                error: format!("http_client gave invalid header value: {value}"),
            });
        };
        headers.insert(key, value);
    }
    Ok(http_response
        .body(get_blob().unwrap_or_default().bytes)
        .unwrap())
}

pub fn get_mime_type(filename: &str) -> String {
    let file_path = Path::new(filename);

    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("octet-stream");

    mime_guess::from_ext(extension)
        .first_or_octet_stream()
        .to_string()
}

/// Serve index.html
pub fn serve_index_html(
    our: &Address,
    directory: &str,
    authenticated: bool,
    local_only: bool,
    paths: Vec<&str>,
) -> anyhow::Result<()> {
    KiRequest::to(("our", "vfs", "distro", "sys"))
        .body(serde_json::to_vec(&VfsRequest {
            path: format!("/{}/pkg/{}/index.html", our.package_id(), directory),
            action: VfsAction::Read,
        })?)
        .send_and_await_response(5)??;

    let Some(blob) = get_blob() else {
        return Err(anyhow::anyhow!("serve_index_html: no index.html blob"));
    };

    let index = String::from_utf8(blob.bytes)?;

    for path in paths {
        bind_http_static_path(
            path,
            authenticated,
            local_only,
            Some("text/html".to_string()),
            index.to_string().as_bytes().to_vec(),
        )?;
    }

    Ok(())
}

/// Serve static files from a given directory by binding all of them
/// in http_server to their filesystem path.
pub fn serve_ui(
    our: &Address,
    directory: &str,
    authenticated: bool,
    local_only: bool,
    paths: Vec<&str>,
) -> anyhow::Result<()> {
    serve_index_html(our, directory, authenticated, local_only, paths)?;

    let initial_path = format!("{}/pkg/{}", our.package_id(), directory);

    let mut queue = VecDeque::new();
    queue.push_back(initial_path.clone());

    while let Some(path) = queue.pop_front() {
        let Ok(directory_response) = KiRequest::to(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&VfsRequest {
                path,
                action: VfsAction::ReadDir,
            })?)
            .send_and_await_response(5)?
        else {
            return Err(anyhow::anyhow!("serve_ui: no response for path"));
        };

        let directory_body = serde_json::from_slice::<VfsResponse>(directory_response.body())?;

        // Determine if it's a file or a directory and handle appropriately
        match directory_body {
            VfsResponse::ReadDir(directory_info) => {
                for entry in directory_info {
                    match entry.file_type {
                        // If it's a file, serve it statically
                        FileType::File => {
                            KiRequest::to(("our", "vfs", "distro", "sys"))
                                .body(serde_json::to_vec(&VfsRequest {
                                    path: entry.path.clone(),
                                    action: VfsAction::Read,
                                })?)
                                .send_and_await_response(5)??;

                            let Some(blob) = get_blob() else {
                                return Err(anyhow::anyhow!(
                                    "serve_ui: no blob for {}",
                                    entry.path
                                ));
                            };

                            let content_type = get_mime_type(&entry.path);

                            bind_http_static_path(
                                entry.path.replace(&initial_path, ""),
                                authenticated, // Must be authenticated
                                local_only,    // Is not local-only
                                Some(content_type),
                                blob.bytes,
                            )?;
                        }
                        FileType::Directory => {
                            // Push the directory onto the queue
                            queue.push_back(entry.path);
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "serve_ui: unexpected response for path: {:?}",
                    directory_body
                ))
            }
        };
    }

    Ok(())
}

pub fn handle_ui_asset_request(our: &Address, directory: &str, path: &str) -> anyhow::Result<()> {
    let parts: Vec<&str> = path.split(&our.process.to_string()).collect();
    let after_process = parts.get(1).unwrap_or(&"");

    let target_path = format!("{}/{}", directory, after_process.trim_start_matches('/'));

    KiRequest::to(("our", "vfs", "distro", "sys"))
        .body(serde_json::to_vec(&VfsRequest {
            path: format!("{}/pkg/{}", our.package_id(), target_path),
            action: VfsAction::Read,
        })?)
        .send_and_await_response(5)??;

    let mut headers = HashMap::new();
    let content_type = get_mime_type(path);
    headers.insert("Content-Type".to_string(), content_type);

    KiResponse::new()
        .body(
            serde_json::json!(HttpResponse {
                status: 200,
                headers,
            })
            .to_string()
            .as_bytes()
            .to_vec(),
        )
        .inherit(true)
        .send()?;

    Ok(())
}

pub fn send_ws_push(channel_id: u32, message_type: WsMessageType, blob: KiBlob) {
    KiRequest::to(("our", "http_server", "distro", "sys"))
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

pub fn open_ws_connection(
    url: String,
    headers: Option<HashMap<String, String>>,
    channel_id: u32,
) -> std::result::Result<(), HttpClientError> {
    let Ok(Ok(Message::Response { body, .. })) =
        KiRequest::to(("our", "http_client", "distro", "sys"))
            .body(
                serde_json::to_vec(&HttpClientAction::WebSocketOpen {
                    url: url.clone(),
                    headers: headers.unwrap_or(HashMap::new()),
                    channel_id,
                })
                .unwrap(),
            )
            .send_and_await_response(5)
    else {
        return Err(HttpClientError::WsOpenFailed { url });
    };
    match serde_json::from_slice(&body) {
        Ok(Ok(HttpClientResponse::WebSocketAck)) => Ok(()),
        Ok(Err(e)) => Err(e),
        _ => Err(HttpClientError::WsOpenFailed { url }),
    }
}

pub fn send_ws_client_push(channel_id: u32, message_type: WsMessageType, blob: KiBlob) {
    KiRequest::to(("our", "http_client", "distro", "sys"))
        .body(
            serde_json::to_vec(&HttpClientAction::WebSocketPush {
                channel_id,
                message_type,
            })
            .unwrap(),
        )
        .blob(blob)
        .send()
        .unwrap()
}

pub fn close_ws_connection(channel_id: u32) -> std::result::Result<(), HttpClientError> {
    let Ok(Ok(Message::Response { body, .. })) =
        KiRequest::to(("our", "http_client", "distro", "sys"))
            .body(
                serde_json::json!(HttpClientAction::WebSocketClose { channel_id })
                    .to_string()
                    .as_bytes()
                    .to_vec(),
            )
            .send_and_await_response(5)
    else {
        return Err(HttpClientError::WsCloseFailed { channel_id });
    };
    match serde_json::from_slice(&body) {
        Ok(Ok(HttpClientResponse::WebSocketAck)) => Ok(()),
        Ok(Err(e)) => Err(e),
        _ => Err(HttpClientError::WsCloseFailed { channel_id }),
    }
}
