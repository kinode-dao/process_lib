use crate::Request;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod directory;
pub mod file;

pub use directory::*;
pub use file::*;

/// IPC body format for requests sent to vfs runtime module.
#[derive(Debug, Serialize, Deserialize)]
pub struct VfsRequest {
    /// path is always prepended by [`crate::PackageId`], the capabilities of the topmost folder are checked
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

pub fn vfs_request<T>(path: T, action: VfsAction) -> Request
where
    T: Into<String>,
{
    Request::new().target(("our", "vfs", "distro", "sys")).body(
        serde_json::to_vec(&VfsRequest {
            path: path.into(),
            action,
        })
        .expect("failed to serialize VfsRequest"),
    )
}

/// Metadata of a path, returns file type and length.
pub fn metadata(path: &str, timeout: Option<u64>) -> Result<FileMetadata, VfsError> {
    let timeout = timeout.unwrap_or(5);

    let message = vfs_request(path, VfsAction::Metadata)
        .send_and_await_response(timeout)
        .unwrap()
        .map_err(|e| VfsError::IOError {
            error: e.to_string(),
            path: path.to_string(),
        })?;

    match parse_response(message.body())? {
        VfsResponse::Metadata(metadata) => Ok(metadata),
        VfsResponse::Err(e) => Err(e),
        _ => Err(VfsError::ParseError {
            error: "unexpected response".to_string(),
            path: path.to_string(),
        }),
    }
}

/// Removes a path, if it's either a directory or a file.
pub fn remove_path(path: &str, timeout: Option<u64>) -> Result<(), VfsError> {
    let meta = metadata(path, timeout)?;

    match meta.file_type {
        FileType::Directory => remove_dir(path, timeout),
        FileType::File => remove_file(path, timeout),
        _ => Err(VfsError::ParseError {
            error: "path is not a file or directory".to_string(),
            path: path.to_string(),
        }),
    }
}

pub fn parse_response(body: &[u8]) -> Result<VfsResponse, VfsError> {
    serde_json::from_slice::<VfsResponse>(body).map_err(|e| VfsError::BadJson {
        error: e.to_string(),
    })
}
