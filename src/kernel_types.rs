use crate::uqbar::process::standard as wit;
use crate::{Address, ProcessId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

//
// process-facing kernel types, used for process
// management and message-passing
// matches types in uqbar.wit
//

pub type Context = Vec<u8>;
pub type NodeId = String; // QNS domain name

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Payload {
    pub mime: Option<String>, // MIME type
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub inherit: bool,
    pub expects_response: Option<u64>, // number of seconds until timeout
    pub ipc: Vec<u8>,
    pub metadata: Option<String>, // JSON-string
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub inherit: bool,
    pub ipc: Vec<u8>,
    pub metadata: Option<String>, // JSON-string
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum Message {
    Request(Request),
    Response((Response, Option<Context>)),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Capability {
    pub issuer: Address,
    pub params: String, // JSON-string
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SignedCapability {
    pub issuer: Address,
    pub params: String,     // JSON-string
    pub signature: Vec<u8>, // signed by the kernel, so we can verify that the kernel issued it
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SendError {
    pub kind: SendErrorKind,
    pub target: Address,
    pub message: Message,
    pub payload: Option<Payload>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SendErrorKind {
    Offline,
    Timeout,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OnExit {
    None,
    Restart,
    Requests(Vec<(Address, Request, Option<Payload>)>),
}

impl OnExit {
    pub fn is_restart(&self) -> bool {
        match self {
            OnExit::None => false,
            OnExit::Restart => true,
            OnExit::Requests(_) => false,
        }
    }
}

/// IPC format for requests sent to kernel runtime module
#[derive(Debug, Serialize, Deserialize)]
pub enum KernelCommand {
    /// RUNTIME ONLY: used to notify the kernel that booting is complete and
    /// all processes have been loaded in from their persisted or bootstrapped state.
    Booted,
    /// Tell the kernel to install and prepare a new process for execution.
    /// The process will not begin execution until the kernel receives a
    /// `RunProcess` command with the same `id`.
    ///
    /// The process that sends this command will be given messaging capabilities
    /// for the new process if `public` is false.
    InitializeProcess {
        id: ProcessId,
        wasm_bytes_handle: u128,
        on_exit: OnExit,
        initial_capabilities: HashSet<SignedCapability>,
        public: bool,
    },
    /// Tell the kernel to run a process that has already been installed.
    /// TODO: in the future, this command could be extended to allow for
    /// resource provision.
    RunProcess(ProcessId),
    /// Kill a running process immediately. This may result in the dropping / mishandling of messages!
    KillProcess(ProcessId),
    /// RUNTIME ONLY: notify the kernel that the runtime is shutting down and it
    /// should gracefully stop and persist the running processes.
    Shutdown,
}

/// IPC format for all KernelCommand responses
#[derive(Debug, Serialize, Deserialize)]
pub enum KernelResponse {
    InitializedProcess,
    InitializeProcessError,
    StartedProcess,
    RunProcessError,
    KilledProcess(ProcessId),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedProcess {
    pub wasm_bytes_handle: u128,
    // pub drive: String,
    // pub full_path: String,
    pub on_exit: OnExit,
    pub capabilities: HashSet<Capability>,
    pub public: bool, // marks if a process allows messages from any process
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VfsRequest {
    pub drive: String,
    pub action: VfsAction,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VfsAction {
    New,
    Add {
        full_path: String,
        entry_type: AddEntryType,
    },
    Rename {
        full_path: String,
        new_full_path: String,
    },
    Delete(String),
    WriteOffset {
        full_path: String,
        offset: u64,
    },
    SetSize {
        full_path: String,
        size: u64,
    },
    GetPath(u128),
    GetHash(String),
    GetEntry(String),
    GetFileChunk {
        full_path: String,
        offset: u64,
        length: u64,
    },
    GetEntryLength(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AddEntryType {
    Dir,
    NewFile,                     //  add a new file to fs and add name in vfs
    ExistingFile { hash: u128 }, //  link an existing file in fs to a new name in vfs
    ZipArchive,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum GetEntryType {
    Dir,
    File,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VfsResponse {
    Ok,
    Err(VfsError),
    GetPath(Option<String>),
    GetHash(Option<u128>),
    GetEntry {
        // file bytes in payload, if entry was a file
        is_file: bool,
        children: Vec<String>,
    },
    GetFileChunk, // chunk in payload
    GetEntryLength(u64),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VfsError {
    BadDriveName,
    BadDescriptor,
    NoCap,
    EntryNotFound,
}

#[allow(dead_code)]
impl VfsError {
    pub fn kind(&self) -> &str {
        match *self {
            VfsError::BadDriveName => "BadDriveName",
            VfsError::BadDescriptor => "BadDescriptor",
            VfsError::NoCap => "NoCap",
            VfsError::EntryNotFound => "EntryNotFound",
        }
    }
}

//
// package types
//

pub type PackageVersion = (u32, u32, u32);

/// the type that gets deserialized from `metadata.json` in a package
#[derive(Debug, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub package: String,
    pub publisher: String,
    pub version: PackageVersion,
    pub wit_version: Option<(u32, u32, u32)>,
    pub description: Option<String>,
    pub website: Option<String>,
}

/// the type that gets deserialized from each entry in the array in `manifest.json`
#[derive(Debug, Serialize, Deserialize)]
pub struct PackageManifestEntry {
    pub process_name: String,
    pub process_wasm_path: String,
    pub on_exit: OnExit,
    pub request_networking: bool,
    pub request_messaging: Option<Vec<String>>,
    pub grant_messaging: Option<Vec<String>>,
    pub public: bool,
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Message::Request(request) => write!(
                f,
                "Request(\n        inherit: {},\n        expects_response: {:?},\n        ipc: {} bytes,\n        metadata: {}\n    )",
                request.inherit,
                request.expects_response,
                request.ipc.len(),
                &request.metadata.as_ref().unwrap_or(&"None".into()),
            ),
            Message::Response((response, context)) => write!(
                f,
                "Response(\n        inherit: {},\n        ipc: {} bytes,\n        metadata: {},\n        context: {} bytes\n    )",
                response.inherit,
                response.ipc.len(),
                &response.metadata.as_ref().unwrap_or(&"None".into()),
                if context.is_none() {
                    0
                } else {
                    context.as_ref().unwrap().len()
                },
            ),
        }
    }
}

//
// conversions between wit types and kernel types (annoying!)
//

pub fn de_wit_address(wit: wit::Address) -> Address {
    Address {
        node: wit.node,
        process: wit.process,
    }
}

pub fn en_wit_address(address: Address) -> wit::Address {
    wit::Address {
        node: address.node,
        process: address.process,
    }
}

pub fn de_wit_request(wit: wit::Request) -> Request {
    Request {
        inherit: wit.inherit,
        expects_response: wit.expects_response,
        ipc: wit.ipc,
        metadata: wit.metadata,
    }
}

pub fn en_wit_request(request: Request) -> wit::Request {
    wit::Request {
        inherit: request.inherit,
        expects_response: request.expects_response,
        ipc: request.ipc,
        metadata: request.metadata,
    }
}

pub fn de_wit_response(wit: wit::Response) -> Response {
    Response {
        inherit: wit.inherit,
        ipc: wit.ipc,
        metadata: wit.metadata,
    }
}

pub fn en_wit_response(response: Response) -> wit::Response {
    wit::Response {
        inherit: response.inherit,
        ipc: response.ipc,
        metadata: response.metadata,
    }
}

pub fn de_wit_payload(wit: Option<wit::Payload>) -> Option<Payload> {
    match wit {
        None => None,
        Some(wit) => Some(Payload {
            mime: wit.mime,
            bytes: wit.bytes,
        }),
    }
}

pub fn en_wit_payload(load: Option<Payload>) -> Option<wit::Payload> {
    match load {
        None => None,
        Some(load) => Some(wit::Payload {
            mime: load.mime,
            bytes: load.bytes,
        }),
    }
}

pub fn de_wit_signed_capability(wit: wit::SignedCapability) -> SignedCapability {
    SignedCapability {
        issuer: Address {
            node: wit.issuer.node,
            process: ProcessId {
                process_name: wit.issuer.process.process_name,
                package_name: wit.issuer.process.package_name,
                publisher_node: wit.issuer.process.publisher_node,
            },
        },
        params: wit.params,
        signature: wit.signature,
    }
}

pub fn en_wit_signed_capability(cap: SignedCapability) -> wit::SignedCapability {
    wit::SignedCapability {
        issuer: en_wit_address(cap.issuer),
        params: cap.params,
        signature: cap.signature,
    }
}

pub fn en_wit_message(message: Message) -> wit::Message {
    match message {
        Message::Request(request) => wit::Message::Request(en_wit_request(request)),
        Message::Response((response, context)) => {
            wit::Message::Response((en_wit_response(response), context))
        }
    }
}

pub fn en_wit_send_error(error: SendError) -> wit::SendError {
    wit::SendError {
        kind: en_wit_send_error_kind(error.kind),
        message: en_wit_message(error.message),
        payload: en_wit_payload(error.payload),
    }
}

pub fn en_wit_send_error_kind(kind: SendErrorKind) -> wit::SendErrorKind {
    match kind {
        SendErrorKind::Offline => wit::SendErrorKind::Offline,
        SendErrorKind::Timeout => wit::SendErrorKind::Timeout,
    }
}
