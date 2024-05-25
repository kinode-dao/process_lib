use crate::kinode::process::standard as wit;
use crate::{Address, ProcessId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

//
// process-facing kernel types, used for process
// management and message-passing
// matches types in kinode.wit
//

pub type Context = Vec<u8>;
pub type NodeId = String; // QNS domain name

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LazyLoadBlob {
    pub mime: Option<String>, // MIME type
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub inherit: bool,
    pub expects_response: Option<u64>, // number of seconds until timeout
    pub body: Vec<u8>,
    pub metadata: Option<String>, // JSON-string
    pub capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub inherit: bool,
    pub body: Vec<u8>,
    pub metadata: Option<String>, // JSON-string
    pub capabilities: Vec<Capability>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SendError {
    pub kind: SendErrorKind,
    pub target: Address,
    pub message: Message,
    pub lazy_load_blob: Option<LazyLoadBlob>,
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
    Requests(Vec<(Address, Request, Option<LazyLoadBlob>)>),
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

/// IPC body format for requests sent to kernel runtime module
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
    ///
    /// All capabilities passed into initial_capabilities must be held by the source
    /// of this message, or the kernel will discard them (silently for now).
    InitializeProcess {
        id: ProcessId,
        wasm_bytes_handle: String,
        wit_version: Option<u32>,
        on_exit: OnExit,
        initial_capabilities: HashSet<Capability>,
        public: bool,
    },
    /// Create an arbitrary capability and grant it to a process.
    GrantCapabilities {
        target: ProcessId,
        capabilities: Vec<Capability>,
    },
    /// Drop capabilities. Does nothing if process doesn't have these caps
    DropCapabilities {
        target: ProcessId,
        capabilities: Vec<Capability>,
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
    /// Ask kernel to produce debugging information
    Debug(KernelPrint),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum KernelPrint {
    ProcessMap,
    Process(ProcessId),
    HasCap { on: ProcessId, cap: Capability },
}

/// IPC body format for all KernelCommand responses
#[derive(Debug, Serialize, Deserialize)]
pub enum KernelResponse {
    InitializedProcess,
    InitializeProcessError,
    StartedProcess,
    RunProcessError,
    KilledProcess(ProcessId),
    Debug(KernelPrintResponse),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum KernelPrintResponse {
    ProcessMap(ProcessMap),
    Process(Option<PersistedProcess>),
    HasCap(Option<bool>),
}

pub type ProcessMap = HashMap<ProcessId, PersistedProcess>;

// NOTE: this is different from the runtime representation of a process
// in that the capabilities are stored as a Vec<(Capability, Vec<u8>)> instead of a HashMap.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedProcess {
    pub wasm_bytes_handle: String,
    pub wit_version: Option<u32>,
    pub on_exit: OnExit,
    pub capabilities: Vec<(Capability, Vec<u8>)>,
    pub public: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StateAction {
    GetState(ProcessId),
    SetState(ProcessId),
    DeleteState(ProcessId),
    Backup,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StateResponse {
    GetState,
    SetState,
    DeleteState,
    Backup,
    Err(StateError),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StateError {
    RocksDBError { action: String, error: String },
    StartupError { action: String },
    BadBytes { action: String },
    BadRequest { error: String },
    BadJson { error: String },
    NotFound { process_id: ProcessId },
    IOError { error: String },
}

#[allow(dead_code)]
impl StateError {
    pub fn kind(&self) -> &str {
        match *self {
            StateError::RocksDBError { .. } => "RocksDBError",
            StateError::StartupError { .. } => "StartupError",
            StateError::BadBytes { .. } => "BadBytes",
            StateError::BadRequest { .. } => "BadRequest",
            StateError::BadJson { .. } => "NoJson",
            StateError::NotFound { .. } => "NotFound",
            StateError::IOError { .. } => "IOError",
        }
    }
}

//
// package types
//

/// Represents the metadata associated with a kinode package, which is an ERC721 compatible token.
/// This is deserialized from the `metadata.json` file in a package.
/// Fields:
/// - `name`: An optional field representing the display name of the package. This does not have to be unique, and is not used for identification purposes.
/// - `description`: An optional field providing a description of the package.
/// - `image`: An optional field containing a URL to an image representing the package.
/// - `external_url`: An optional field containing a URL for more information about the package. For example, a link to the github repository.
/// - `animation_url`: An optional field containing a URL to an animation or video representing the package.
/// - `properties`: A requried field containing important information about the package.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Erc721Metadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub external_url: Option<String>,
    pub animation_url: Option<String>,
    pub properties: Erc721Properties,
}

/// Represents critical fields of a kinode package in an ERC721 compatible format.
/// This follows the [ERC1155](https://github.com/ethereum/ercs/blob/master/ERCS/erc-1155.md#erc-1155-metadata-uri-json-schema) metadata standard.
///
/// Fields:
/// - `package_name`: The unique name of the package, used in the `PackageId`, e.g. `package_name:publisher`.
/// - `publisher`: The KNS identity of the package publisher used in the `PackageId`, e.g. `package_name:publisher`
/// - `current_version`: A string representing the current version of the package, e.g. `1.0.0`.
/// - `mirrors`: A list of NodeIds where the package can be found, providing redundancy.
/// - `code_hashes`: A map from version names to their respective SHA-256 hashes.
/// - `license`: An optional field containing the license of the package.
/// - `screenshots`: An optional field containing a list of URLs to screenshots of the package.
/// - `wit_version`: An optional field containing the version of the WIT standard that the package adheres to.
/// - `dependencies`: An optional field containing a list of `PackageId`s: API dependencies.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Erc721Properties {
    pub package_name: String,
    pub publisher: String,
    pub current_version: String,
    pub mirrors: Vec<NodeId>,
    pub code_hashes: HashMap<String, String>,
    pub license: Option<String>,
    pub screenshots: Option<Vec<String>>,
    pub wit_version: Option<u32>,
    pub dependencies: Option<Vec<String>>,
}

/// the type that gets deserialized from each entry in the array in `manifest.json`
#[derive(Debug, Serialize, Deserialize)]
pub struct PackageManifestEntry {
    pub process_name: String,
    pub process_wasm_path: String,
    pub on_exit: OnExit,
    pub request_networking: bool,
    pub request_capabilities: Vec<serde_json::Value>,
    pub grant_capabilities: Vec<serde_json::Value>,
    pub public: bool,
}

/// the type that gets deserialized from a `scripts.json` object
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DotScriptsEntry {
    pub root: bool,
    pub public: bool,
    pub request_networking: bool,
    pub request_capabilities: Option<Vec<serde_json::Value>>,
    pub grant_capabilities: Option<Vec<serde_json::Value>>,
    pub wit_version: Option<u32>,
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Message::Request(request) => write!(
                f,
                "Request(\n        inherit: {},\n        expects_response: {:?},\n        body: {} bytes,\n        metadata: {}\n    )",
                request.inherit,
                request.expects_response,
                request.body.len(),
                &request.metadata.as_ref().unwrap_or(&"None".into()),
            ),
            Message::Response((response, context)) => write!(
                f,
                "Response(\n        inherit: {},\n        body: {} bytes,\n        metadata: {},\n        context: {} bytes\n    )",
                response.inherit,
                response.body.len(),
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
        body: wit.body,
        metadata: wit.metadata,
        capabilities: wit
            .capabilities
            .into_iter()
            .map(de_wit_capability)
            .collect(),
    }
}

pub fn en_wit_request(request: Request) -> wit::Request {
    wit::Request {
        inherit: request.inherit,
        expects_response: request.expects_response,
        body: request.body,
        metadata: request.metadata,
        capabilities: request
            .capabilities
            .into_iter()
            .map(en_wit_capability)
            .collect(),
    }
}

pub fn de_wit_response(wit: wit::Response) -> Response {
    Response {
        inherit: wit.inherit,
        body: wit.body,
        metadata: wit.metadata,
        capabilities: wit
            .capabilities
            .into_iter()
            .map(de_wit_capability)
            .collect(),
    }
}

pub fn en_wit_response(response: Response) -> wit::Response {
    wit::Response {
        inherit: response.inherit,
        body: response.body,
        metadata: response.metadata,
        capabilities: response
            .capabilities
            .into_iter()
            .map(en_wit_capability)
            .collect(),
    }
}

pub fn de_wit_blob(wit: Option<wit::LazyLoadBlob>) -> Option<LazyLoadBlob> {
    match wit {
        None => None,
        Some(wit) => Some(LazyLoadBlob {
            mime: wit.mime,
            bytes: wit.bytes,
        }),
    }
}

pub fn en_wit_blob(load: Option<LazyLoadBlob>) -> Option<wit::LazyLoadBlob> {
    match load {
        None => None,
        Some(load) => Some(wit::LazyLoadBlob {
            mime: load.mime,
            bytes: load.bytes,
        }),
    }
}

pub fn de_wit_capability(wit: wit::Capability) -> Capability {
    Capability {
        issuer: Address {
            node: wit.issuer.node,
            process: ProcessId {
                process_name: wit.issuer.process.process_name,
                package_name: wit.issuer.process.package_name,
                publisher_node: wit.issuer.process.publisher_node,
            },
        },
        params: wit.params,
    }
}

pub fn en_wit_capability(cap: Capability) -> wit::Capability {
    wit::Capability {
        issuer: en_wit_address(cap.issuer),
        params: cap.params,
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
        target: en_wit_address(error.target),
        message: en_wit_message(error.message),
        lazy_load_blob: en_wit_blob(error.lazy_load_blob),
    }
}

pub fn en_wit_send_error_kind(kind: SendErrorKind) -> wit::SendErrorKind {
    match kind {
        SendErrorKind::Offline => wit::SendErrorKind::Offline,
        SendErrorKind::Timeout => wit::SendErrorKind::Timeout,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageType {
    Request,
    Response,
}
