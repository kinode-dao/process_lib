use crate::{Message, Request};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod directory;
pub mod file;

pub use directory::*;
pub use file::*;

/// IPC body format for requests sent to vfs runtime module
#[derive(Debug, Serialize, Deserialize)]
pub struct VfsRequest {
    /// path is always prepended by package_id, the capabilities of the topmost folder are checked
    /// "/your_package:publisher.os/drive_folder/another_folder_or_file"
    pub path: String,
    pub action: VfsAction,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VfsAction {
    CreateDrive,
    CreateDir,
    CreateDirAll,
    CreateFile,
    OpenFile { create: bool },
    CloseFile,
    Write,
    WriteAll,
    Append,
    SyncAll,
    Read,
    ReadDir,
    ReadToEnd,
    ReadExact(u64),
    ReadToString,
    Seek { seek_from: SeekFrom },
    RemoveFile,
    RemoveDir,
    RemoveDirAll,
    Rename { new_path: String },
    Metadata,
    AddZip,
    CopyFile { new_path: String },
    Len,
    SetLen(u64),
    Hash,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum FileType {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub file_type: FileType,
    pub len: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DirEntry {
    pub path: String,
    pub file_type: FileType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VfsResponse {
    Ok,
    Err(VfsError),
    Read,
    SeekFrom(u64),
    ReadDir(Vec<DirEntry>),
    ReadToString(String),
    Metadata(FileMetadata),
    Len(u64),
    Hash([u8; 32]),
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum VfsError {
    #[error("vfs: No capability for action {action} at path {path}")]
    NoCap { action: String, path: String },
    #[error("vfs: Bytes blob required for {action} at path {path}")]
    BadBytes { action: String, path: String },
    #[error("vfs: bad request error: {error}")]
    BadRequest { error: String },
    #[error("vfs: error parsing path: {path}, error: {error}")]
    ParseError { error: String, path: String },
    #[error("vfs: IO error: {error}, at path {path}")]
    IOError { error: String, path: String },
    #[error("vfs: kernel capability channel error: {error}")]
    CapChannelFail { error: String },
    #[error("vfs: Bad JSON blob: {error}")]
    BadJson { error: String },
    #[error("vfs: File not found at path {path}")]
    NotFound { path: String },
    #[error("vfs: Creating directory failed at path: {path}: {error}")]
    CreateDirError { path: String, error: String },
}

#[allow(dead_code)]
impl VfsError {
    pub fn kind(&self) -> &str {
        match *self {
            VfsError::NoCap { .. } => "NoCap",
            VfsError::BadBytes { .. } => "BadBytes",
            VfsError::BadRequest { .. } => "BadRequest",
            VfsError::ParseError { .. } => "ParseError",
            VfsError::IOError { .. } => "IOError",
            VfsError::CapChannelFail { .. } => "CapChannelFail",
            VfsError::BadJson { .. } => "NoJson",
            VfsError::NotFound { .. } => "NotFound",
            VfsError::CreateDirError { .. } => "CreateDirError",
        }
    }
}

/// Metadata of a path, returns file type and length.
pub fn metadata(path: &str) -> anyhow::Result<FileMetadata> {
    let request = VfsRequest {
        path: path.to_string(),
        action: VfsAction::Metadata,
    };
    let message = Request::new()
        .target(("our", "vfs", "distro", "sys"))
        .body(serde_json::to_vec(&request)?)
        .send_and_await_response(5)?;

    match message {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<VfsResponse>(&body)?;
            match response {
                VfsResponse::Metadata(metadata) => Ok(metadata),
                VfsResponse::Err(e) => Err(e.into()),
                _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
            }
        }
        _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
    }
}

/// Removes a path, if it's either a directory or a file.
pub fn remove_path(path: &str) -> anyhow::Result<()> {
    let meta = metadata(path)?;
    match meta.file_type {
        FileType::Directory => remove_dir(path),
        FileType::File => remove_file(path),
        _ => Err(anyhow::anyhow!(
            "vfs: path is not a file or directory: {}",
            path
        )),
    }
}
