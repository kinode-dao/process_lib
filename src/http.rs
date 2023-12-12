use crate::kernel_types::Payload;
use crate::{Message, Request as uqRequest, Response as uqResponse};
pub use http::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

//
// these types are a copy of the types used in http module of runtime.
//

/// HTTP Request type that can be shared over WASM boundary to apps.
/// This is the one you receive from the `http_server:sys:uqbar` service.
#[derive(Debug, Serialize, Deserialize)]
pub enum HttpServerRequest {
    Http(IncomingHttpRequest),
    /// Processes will receive this kind of request when a client connects to them.
    /// If a process does not want this websocket open, they should issue a *request*
    /// containing a [`type@HttpServerAction::WebSocketClose`] message and this channel ID.
    WebSocketOpen(u32),
    /// Processes can both SEND and RECEIVE this kind of request
    /// (send as [`type@HttpServerAction::WebSocketPush`]).
    /// When received, will contain the message bytes as payload.
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
    pub source_socket_addr: Option<String>, // will parse to SocketAddr
    pub method: String,                     // will parse to http::Method
    pub raw_path: String,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    // BODY is stored in the payload, as bytes
}

/// HTTP Request type that can be shared over WASM boundary to apps.
/// This is the one you send to the `http_client:sys:uqbar` service.
#[derive(Debug, Serialize, Deserialize)]
pub struct OutgoingHttpRequest {
    pub method: String,          // must parse to http::Method
    pub version: Option<String>, // must parse to http::Version
    pub url: String,             // must parse to url::Url
    pub headers: HashMap<String, String>,
    // BODY is stored in the payload, as bytes
    // TIMEOUT is stored in the message expect_response
}

/// HTTP Response type that can be shared over WASM boundary to apps.
/// Respond to [`IncomingHttpRequest`] with this type.
#[derive(Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    // BODY is stored in the payload, as bytes
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcResponseBody {
    pub ipc: Vec<u8>,
    pub payload: Option<Payload>,
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum HttpClientError {
    #[error("http_client: request could not be parsed to HttpRequest: {}.", req)]
    BadRequest { req: String },
    #[error("http_client: http method not supported: {}", method)]
    BadMethod { method: String },
    #[error("http_client: url could not be parsed: {}", url)]
    BadUrl { url: String },
    #[error("http_client: http version not supported: {}", version)]
    BadVersion { version: String },
    #[error("http_client: failed to execute request {}", error)]
    RequestFailed { error: String },
}

/// Request type sent to `http_server:sys:uqbar` in order to configure it.
/// You can also send [`type@HttpServerAction::WebSocketPush`], which
/// allows you to push messages across an existing open WebSocket connection.
///
/// If a response is expected, all HttpServerActions will return a Response
/// with the shape Result<(), HttpServerActionError> serialized to JSON.
#[derive(Debug, Serialize, Deserialize)]
pub enum HttpServerAction {
    /// Bind expects a payload if and only if `cache` is TRUE. The payload should
    /// be the static file to serve at this path.
    Bind {
        path: String,
        /// Set whether the HTTP request needs a valid login cookie, AKA, whether
        /// the user needs to be logged in to access this path.
        authenticated: bool,
        /// Set whether requests can be fielded from anywhere, or only the loopback address.
        local_only: bool,
        /// Set whether to bind the payload statically to this path. That is, take the
        /// payload bytes and serve them as the response to any request to this path.
        cache: bool,
    },
    /// SecureBind expects a payload if and only if `cache` is TRUE. The payload should
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
        /// Set whether to bind the payload statically to this path. That is, take the
        /// payload bytes and serve them as the response to any request to this path.
        cache: bool,
    },
    /// Processes will RECEIVE this kind of request when a client connects to them.
    /// If a process does not want this websocket open, they should issue a *request*
    /// containing a [`type@HttpServerAction::WebSocketClose`] message and this channel ID.
    WebSocketOpen(u32),
    /// When sent, expects a payload containing the WebSocket message bytes to send.
    WebSocketPush {
        channel_id: u32,
        message_type: WsMessageType,
    },
    /// Sending will close a socket the process controls.
    WebSocketClose(u32),
}

/// The possible message types for WebSocketPush. Ping and Pong are limited to 125 bytes
/// by the WebSockets protocol. Text will be sent as a Text frame, with the payload bytes
/// being the UTF-8 encoding of the string. Binary will be sent as a Binary frame containing
/// the unmodified payload bytes.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum WsMessageType {
    Text,
    Binary,
    Ping,
    Pong,
}

/// Part of the Response type issued by http_server
#[derive(Error, Debug, Serialize, Deserialize)]
pub enum HttpServerError {
    #[error(
        "http_server: request could not be parsed to HttpServerAction: {}.",
        req
    )]
    BadRequest { req: String },
    #[error("http_server: action expected payload")]
    NoPayload,
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

impl IncomingHttpRequest {
    pub fn url(&self) -> anyhow::Result<url::Url> {
        url::Url::parse(&self.raw_path).map_err(|e| anyhow::anyhow!("couldn't parse url: {:?}", e))
    }

    pub fn method(&self) -> anyhow::Result<http::Method> {
        http::Method::from_bytes(self.method.as_bytes())
            .map_err(|e| anyhow::anyhow!("couldn't parse method: {:?}", e))
    }

    pub fn path(&self) -> anyhow::Result<String> {
        let url = url::Url::parse(&self.raw_path)?;
        // skip the first path segment, which is the process ID.
        Ok(url
            .path_segments()
            .ok_or(anyhow::anyhow!("url path missing process ID!"))?
            .skip(1)
            .collect())
    }
}

/// Register a new path with the HTTP server. This will cause the HTTP server to
/// forward any requests on this path to the calling process. Requests will be
/// given in the form of `Result<(), HttpServerError>`
pub fn bind_http_path<T>(path: T, authenticated: bool, local_only: bool) -> anyhow::Result<()>
where
    T: Into<String>,
{
    let res = uqRequest::new()
        .target(("our", "http_server", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&HttpServerAction::Bind {
            path: path.into(),
            authenticated,
            local_only,
            cache: false,
        })?)
        .send_and_await_response(5)?;
    match res {
        Ok(Message::Response { ipc, .. }) => {
            let resp: std::result::Result<(), HttpServerError> = serde_json::from_slice(&ipc)?;
            resp.map_err(|e| anyhow::anyhow!(e))
        }
        _ => Err(anyhow::anyhow!("http_server: couldn't bind path")),
    }
}

/// Register a new path with the HTTP server, and serve a static file from it.
/// The server will respond to GET requests on this path with the given file.
pub fn bind_http_static_path<T>(
    path: T,
    authenticated: bool,
    local_only: bool,
    content_type: Option<String>,
    content: Vec<u8>,
) -> anyhow::Result<()>
where
    T: Into<String>,
{
    let res = uqRequest::new()
        .target(("our", "http_server", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&HttpServerAction::Bind {
            path: path.into(),
            authenticated,
            local_only,
            cache: true,
        })?)
        .payload(crate::uqbar::process::standard::Payload {
            mime: content_type,
            bytes: content,
        })
        .send_and_await_response(5)?;
    match res {
        Ok(Message::Response { ipc, .. }) => {
            let resp: std::result::Result<(), HttpServerError> = serde_json::from_slice(&ipc)?;
            resp.map_err(|e| anyhow::anyhow!(e))
        }
        _ => Err(anyhow::anyhow!("http_server: couldn't bind path")),
    }
}

/// Send an HTTP response to the incoming HTTP request.
pub fn send_response(
    status: StatusCode,
    headers: Option<HashMap<String, String>>,
    body: Vec<u8>,
) -> anyhow::Result<()> {
    uqResponse::new()
        .ipc(serde_json::to_vec(&HttpResponse {
            status: status.as_u16(),
            headers: headers.unwrap_or_default(),
        })?)
        .payload_bytes(body)
        .send()
}

/// Fire off an HTTP request. If a timeout is given, the response will
/// come in the main event loop, otherwise none will be given.
pub fn send_request(
    method: Method,
    url: url::Url,
    headers: Option<HashMap<String, String>>,
    timeout: Option<u64>,
    body: Vec<u8>,
) -> anyhow::Result<()> {
    let req = uqRequest::new()
        .target(("our", "http_client", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&OutgoingHttpRequest {
            method: method.to_string(),
            version: None,
            url: url.to_string(),
            headers: headers.unwrap_or_default(),
        })?)
        .payload_bytes(body);
    if let Some(timeout) = timeout {
        req.expects_response(timeout).send()
    } else {
        req.send()
    }
}

/// Make an HTTP request using http_client and await its response.
pub fn send_request_await_response(
    method: Method,
    url: url::Url,
    headers: Option<HashMap<String, String>>,
    timeout: u64,
    body: Vec<u8>,
) -> std::result::Result<HttpResponse, HttpClientError> {
    let res = uqRequest::new()
        .target(("our", "http_client", "sys", "uqbar"))
        .ipc(
            serde_json::to_vec(&OutgoingHttpRequest {
                method: method.to_string(),
                version: None,
                url: url.to_string(),
                headers: headers.unwrap_or_default(),
            })
            .map_err(|e| HttpClientError::BadRequest {
                req: format!("{e:?}"),
            })?,
        )
        .payload_bytes(body)
        .send_and_await_response(timeout)
        .map_err(|e| HttpClientError::RequestFailed {
            error: e.to_string(),
        })?;
    match res {
        Ok(Message::Response { ipc, .. }) => {
            serde_json::from_slice(&ipc).map_err(|e| HttpClientError::RequestFailed {
                error: format!("http_client gave unparsable response: {e}"),
            })
        }
        _ => Err(HttpClientError::RequestFailed {
            error: format!("http_client timed out"),
        }),
    }
}
