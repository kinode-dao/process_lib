use crate::{Address, LazyLoadBlob, Message, _wit_message_to_message};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct SendError {
    pub kind: SendErrorKind,
    pub target: Address,
    pub message: Message,
    pub lazy_load_blob: Option<LazyLoadBlob>,
    pub context: Option<Vec<u8>>,
}

impl SendError {
    pub fn kind(&self) -> &SendErrorKind {
        &self.kind
    }
    pub fn target(&self) -> &Address {
        &self.target
    }
    pub fn message(&self) -> &Message {
        &self.message
    }
    pub fn blob(&self) -> Option<&LazyLoadBlob> {
        self.lazy_load_blob.as_ref()
    }
    pub fn context(&self) -> Option<&[u8]> {
        self.context.as_deref()
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SendErrorKind {
    Offline,
    Timeout,
}

impl SendErrorKind {
    pub fn is_offline(&self) -> bool {
        matches!(self, SendErrorKind::Offline)
    }
    pub fn is_timeout(&self) -> bool {
        matches!(self, SendErrorKind::Timeout)
    }
}

pub fn _wit_send_error_to_send_error(
    send_err: crate::kinode::process::standard::SendError,
    context: Option<Vec<u8>>,
) -> SendError {
    SendError {
        kind: match send_err.kind {
            crate::kinode::process::standard::SendErrorKind::Offline => SendErrorKind::Offline,
            crate::kinode::process::standard::SendErrorKind::Timeout => SendErrorKind::Timeout,
        },
        target: send_err.target.clone(),
        message: _wit_message_to_message(send_err.target, send_err.message),
        lazy_load_blob: send_err.lazy_load_blob,
        context,
    }
}
