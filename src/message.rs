use crate::*;

/// The basic message type. A message is either a request or a response. Best
/// practice when handling a message is to do this:
/// 1. Match on whether it's a request or a response
/// 2. Match on who the message is from (the `source`)
/// 3. Parse and interpret the `ipc`, `metadata`, and/or `context` based on
/// who the message is from and what your process expects from them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Request {
        source: Address,
        expects_response: Option<u64>,
        ipc: Vec<u8>,
        metadata: Option<String>,
    },
    Response {
        source: Address,
        ipc: Vec<u8>,
        metadata: Option<String>,
        context: Option<Vec<u8>>,
    },
}

impl Message {
    /// Get the source of a message.
    pub fn source(&self) -> &Address {
        match self {
            Message::Request { source, .. } => source,
            Message::Response { source, .. } => source,
        }
    }
    /// Get the IPC of a message.
    pub fn ipc(&self) -> &[u8] {
        match self {
            Message::Request { ipc, .. } => ipc,
            Message::Response { ipc, .. } => ipc,
        }
    }
    /// Get the metadata of a message.
    pub fn metadata(&self) -> Option<&str> {
        match self {
            Message::Request { metadata, .. } => metadata.as_ref().map(|s| s.as_str()),
            Message::Response { metadata, .. } => metadata.as_ref().map(|s| s.as_str()),
        }
    }
    /// Get the context of a message.
    pub fn context(&self) -> Option<&[u8]> {
        match self {
            Message::Request { .. } => None,
            Message::Response { context, .. } => context.as_ref().map(|s| s.as_slice()),
        }
    }
    /// Get the payload of a message, if any.
    pub fn payload(&self) -> Option<Payload> {
        crate::get_payload()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SendErrorKind {
    Offline,
    Timeout,
}

impl SendErrorKind {
    pub fn is_offline(&self) -> bool {
        match self {
            SendErrorKind::Offline => true,
            _ => false,
        }
    }
    pub fn is_timeout(&self) -> bool {
        match self {
            SendErrorKind::Timeout => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SendError {
    pub kind: SendErrorKind,
    pub message: Message,
    pub payload: Option<Payload>,
    pub context: Option<Vec<u8>>,
}

impl SendError {
    pub fn kind(&self) -> &SendErrorKind {
        &self.kind
    }
    pub fn message(&self) -> &Message {
        &self.message
    }
    pub fn payload(&self) -> Option<&Payload> {
        self.payload.as_ref()
    }
    pub fn context(&self) -> Option<&[u8]> {
        self.context.as_ref().map(|s| s.as_slice())
    }
}

pub fn wit_message_to_message(
    source: Address,
    message: crate::uqbar::process::standard::Message,
) -> Message {
    match message {
        crate::uqbar::process::standard::Message::Request(req) => Message::Request {
            source,
            expects_response: req.expects_response,
            ipc: req.ipc,
            metadata: req.metadata,
        },
        crate::uqbar::process::standard::Message::Response((resp, context)) => Message::Response {
            source,
            ipc: resp.ipc,
            metadata: resp.metadata,
            context,
        },
    }
}
