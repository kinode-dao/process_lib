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
        capabilities: Vec<Capability>,
    },
    Response {
        source: Address,
        ipc: Vec<u8>,
        metadata: Option<String>,
        context: Option<Vec<u8>>,
        capabilities: Vec<Capability>,
    },
}

impl Message {
    pub fn is_request(&self) -> bool {
        match self {
            Message::Request { .. } => true,
            Message::Response { .. } => false,
        }
    }
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
    /// Get the blob of a message, if any.
    pub fn blob(&self) -> Option<Blob> {
        crate::get_blob()
    }

    /// Get the capabilities of a message.
    pub fn capabilities(&self) -> &Vec<Capability> {
        match self {
            Message::Request { capabilities, .. } => capabilities,
            Message::Response { capabilities, .. } => capabilities,
        }
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
    pub blob: Option<Blob>,
    pub context: Option<Vec<u8>>,
}

impl SendError {
    pub fn kind(&self) -> &SendErrorKind {
        &self.kind
    }
    pub fn message(&self) -> &Message {
        &self.message
    }
    pub fn blob(&self) -> Option<&Blob> {
        self.blob.as_ref()
    }
    pub fn context(&self) -> Option<&[u8]> {
        self.context.as_ref().map(|s| s.as_slice())
    }
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            SendErrorKind::Offline => write!(f, "Offline"),
            SendErrorKind::Timeout => write!(f, "Timeout"),
        }
    }
}

impl std::error::Error for SendError {
    fn description(&self) -> &str {
        match &self.kind {
            SendErrorKind::Offline => "Offline",
            SendErrorKind::Timeout => "Timeout",
        }
    }
}

pub fn wit_message_to_message(
    source: Address,
    message: crate::nectar::process::standard::Message,
) -> Message {
    match message {
        crate::nectar::process::standard::Message::Request(req) => Message::Request {
            source,
            expects_response: req.expects_response,
            ipc: req.ipc,
            metadata: req.metadata,
            capabilities: req.capabilities,
        },
        crate::nectar::process::standard::Message::Response((resp, context)) => Message::Response {
            source,
            ipc: resp.ipc,
            metadata: resp.metadata,
            context,
            capabilities: resp.capabilities,
        },
    }
}
