use crate::{get_payload, Message, PackageId, Request};
use serde::{Deserialize, Serialize};

/// IPC format for requests sent to vfs runtime module
#[derive(Debug, Serialize, Deserialize)]
pub struct VfsRequest {
    /// path is always prepended by package_id, the capabilities of the topmost folder are checked
    /// "/your_package:publisher.uq/drive_folder/another_folder_or_file"
    pub path: String,
    pub action: VfsAction,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VfsAction {
    CreateDrive,
    CreateDir,
    CreateDirAll,
    CreateFile,
    OpenFile,
    CloseFile,
    WriteAll,
    Write,
    ReWrite,
    WriteAt(u64),
    Append,
    SyncAll,
    Read,
    ReadToEnd,
    ReadDir,
    ReadExact(u64),
    ReadToString,
    Seek(SeekFrom),
    RemoveFile,
    RemoveDir,
    RemoveDirAll,
    Rename(String),
    Metadata,
    AddZip,
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct DirEntry {
    pub path: String,
    pub file_type: FileType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VfsResponse {
    Ok,
    Err(VfsError),
    Read,
    ReadDir(Vec<DirEntry>),
    ReadToString(String),
    Metadata(FileMetadata),
    Len(u64),
    Hash([u8; 32]),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VfsError {
    NoCap { action: String, path: String },
    BadBytes { action: String, path: String },
    BadRequest { error: String },
    ParseError { error: String, path: String },
    IOError { error: String, path: String },
    CapChannelFail { error: String },
    BadJson { error: String },
    NotFound { path: String },
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

/// Creates a drive with path "/package_id/drive", gives you read and write caps.
/// Will only work on the same package_id as you're calling it from, unless you
/// have root capabilities.
pub fn create_drive(package_id: PackageId, drive: &str) -> anyhow::Result<String> {
    let path = format!("/{}/{}", package_id.to_string(), drive);
    let res = Request::new()
        .target(("our", "vfs", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&VfsRequest {
            path: path.clone(),
            action: VfsAction::CreateDrive,
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { ipc, .. }) => {
            let response = serde_json::from_slice::<VfsResponse>(&ipc)?;
            match response {
                VfsResponse::Ok => Ok(path),
                VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: create drive error: {:?}", e)),
                _ => Err(anyhow::anyhow!("vfs: unexpected response")),
            }
        }
        _ => return Err(anyhow::anyhow!("vfs: unexpected message")),
    }
}

/// Opens or creates a file at path.
/// If trying to create an existing file, will just open it.
pub fn open_file(path: &str, create: bool) -> anyhow::Result<File> {
    let action = match create {
        true => VfsAction::CreateFile,
        false => VfsAction::OpenFile,
    };

    let request = VfsRequest {
        path: path.to_string(),
        action,
    };
    let message = Request::new()
        .target(("our", "vfs", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&request)?)
        .send_and_await_response(5)?;

    match message {
        Ok(Message::Response { ipc, .. }) => {
            let response = serde_json::from_slice::<VfsResponse>(&ipc)?;
            match response {
                VfsResponse::Ok => Ok(File {
                    path: path.to_string(),
                }),
                VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: open file error: {:?}", e)),
                _ => Err(anyhow::anyhow!("vfs: unexpected response")),
            }
        }
        _ => Err(anyhow::anyhow!("vfs: unexpected message")),
    }
}

pub struct File {
    pub path: String,
}

impl File {
    /// Reads the entire file, from start position, into given buffer.
    /// Returns the number of bytes read.
    pub fn read_from_start(&self, buffer: &mut [u8]) -> anyhow::Result<usize> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::Read,
        };
        let message = Request::new()
            .target(("our", "vfs", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&ipc)?;
                match response {
                    VfsResponse::Read => {
                        let data = match get_payload() {
                            Some(bytes) => bytes.bytes,
                            None => return Err(anyhow::anyhow!("vfs: no read payload")),
                        };
                        let len = std::cmp::min(data.len(), buffer.len());
                        buffer[..len].copy_from_slice(&data[..len]);
                        Ok(len)
                    }
                    VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: read error: {:?}", e)),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response")),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message (not response)")),
        }
    }

    /// Read to buffer from current position.
    pub fn read(&self, buffer: &mut [u8]) -> anyhow::Result<usize> {
        let length = buffer.len();
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::ReadExact(length as u64),
        };
        let message = Request::new()
            .target(("our", "vfs", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&ipc)?;
                match response {
                    VfsResponse::Read => {
                        let data = match get_payload() {
                            Some(bytes) => bytes.bytes,
                            None => return Err(anyhow::anyhow!("vfs: no read payload")),
                        };
                        let len = std::cmp::min(data.len(), buffer.len());
                        buffer[..len].copy_from_slice(&data[..len]);
                        Ok(len)
                    }
                    VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: read error: {:?}", e)),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response")),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message (not response)")),
        }
    }

    /// Overwrites starting from position 0, truncates file to input buffer length.
    pub fn write_from_start(&self, buffer: &[u8]) -> anyhow::Result<()> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::ReWrite,
        };

        let message = Request::new()
            .target(("our", "vfs", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&request)?)
            .payload_bytes(buffer)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&ipc)?;
                match response {
                    VfsResponse::Ok => Ok(()),
                    VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: write error: {:?}", e)),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response")),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message")),
        }
    }

    /// Write buffer to file at current position, overwriting any existing data.
    pub fn write(&mut self, buffer: &[u8]) -> anyhow::Result<()> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::Write,
        };
        let message = Request::new()
            .target(("our", "vfs", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&request)?)
            .payload_bytes(buffer)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&ipc)?;
                match response {
                    VfsResponse::Ok => Ok(()),
                    VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: write error: {:?}", e)),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response")),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message")),
        }
    }

    /// Seek file to position
    pub fn seek(&mut self, pos: SeekFrom) -> anyhow::Result<u64> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::Seek(pos),
        };
        let message = Request::new()
            .target(("our", "vfs", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&ipc)?;
                match response {
                    VfsResponse::Ok => Ok(0), // Replace with actual position
                    VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: seek error: {:?}", e)),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response")),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message")),
        }
    }

    /// Set file length, if given size > underlying file, fills it with 0s.
    pub fn set_len(&mut self, size: u64) -> anyhow::Result<()> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::SetLen(size),
        };
        let message = Request::new()
            .target(("our", "vfs", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&ipc)?;
                match response {
                    VfsResponse::Ok => Ok(()),
                    VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: set_len error: {:?}", e)),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response")),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message")),
        }
    }

    /// Metadata of a path, returns file type and length.
    pub fn metadata(&self) -> anyhow::Result<FileMetadata> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::Metadata,
        };
        let message = Request::new()
            .target(("our", "vfs", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&ipc)?;
                match response {
                    VfsResponse::Metadata(metadata) => Ok(metadata),
                    VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: metadata error: {:?}", e)),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response")),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message")),
        }
    }

    /// Syncs path file buffers to disk.
    pub fn sync_all(&self) -> anyhow::Result<()> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::SyncAll,
        };
        let message = Request::new()
            .target(("our", "vfs", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&ipc)?;
                match response {
                    VfsResponse::Ok => Ok(()),
                    VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: sync error: {:?}", e)),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response")),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message")),
        }
    }
}

/// Opens or creates a directory at path.
/// If trying to create an existing file, will just open it.
pub fn open_dir(path: &str, create: bool) -> anyhow::Result<Directory> {
    if !create {
        return Ok(Directory {
            path: path.to_string(),
        });
    }
    let request = VfsRequest {
        path: path.to_string(),
        action: VfsAction::CreateDir,
    };

    let message = Request::new()
        .target(("our", "vfs", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&request)?)
        .send_and_await_response(5)?;

    match message {
        Ok(Message::Response { ipc, .. }) => {
            let response = serde_json::from_slice::<VfsResponse>(&ipc)?;
            match response {
                VfsResponse::Ok => Ok(Directory {
                    path: path.to_string(),
                }),
                VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: open directory error: {:?}", e)),
                _ => Err(anyhow::anyhow!("vfs: unexpected response")),
            }
        }
        _ => Err(anyhow::anyhow!("vfs: unexpected message")),
    }
}

pub struct Directory {
    path: String,
}

impl Directory {
    /// Iterates through children of directory, returning a vector of DirEntries.
    /// DirEntries contain the path and file type of each child.
    pub fn read(&self) -> anyhow::Result<Vec<DirEntry>> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::ReadDir,
        };
        let message = Request::new()
            .target(("our", "vfs", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&ipc)?;
                match response {
                    VfsResponse::ReadDir(entries) => Ok(entries),
                    VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: read_dir error: {:?}", e)),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response")),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message")),
        }
    }
}
