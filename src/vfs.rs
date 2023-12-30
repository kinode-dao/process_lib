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
    OpenFile { create: bool },
    CloseFile,
    Write,
    WriteAt,
    Append,
    SyncAll,
    Read,
    ReadDir,
    ReadToEnd,
    ReadExact(u64),
    ReadToString,
    Seek { seek_from: SeekFrom},
    RemoveFile,
    RemoveDir,
    RemoveDirAll,
    Rename { new_path: String},
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
    SeekFrom(u64),
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

/// Opens a file at path, if no file at path, creates one if boolean create is true.
pub fn open_file(path: &str, create: bool) -> anyhow::Result<File> {
    let request = VfsRequest {
        path: path.to_string(),
        action: VfsAction::OpenFile { create },
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

/// Creates a file at path, if file found at path, truncates it to 0.
pub fn create_file(path: &str) -> anyhow::Result<File> {
    let request = VfsRequest {
        path: path.to_string(),
        action: VfsAction::CreateFile,
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
    /// Reads the entire file, from start position.
    /// Returns a vector of bytes.
    pub fn read(&self) -> anyhow::Result<Vec<u8>> {
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
                        Ok(data)
                    }
                    VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: read error: {:?}", e)),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response")),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message (not response)")),
        }
    }

    /// Reads the entire file, from start position, into buffer.
    /// Returns the amount of bytes read.
    pub fn read_into(&self, buffer: &mut [u8]) -> anyhow::Result<usize> {
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

    /// Read into buffer from current cursor position
    /// Returns the amount of bytes read.
    pub fn read_at(&self, buffer: &mut [u8]) -> anyhow::Result<usize> {
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

    /// Write entire slice as the new file. 
    /// Truncates anything that existed at path before.
    pub fn write(&self, buffer: &[u8]) -> anyhow::Result<()> {
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

    /// Write buffer to file at current position, overwriting any existing data.
    pub fn write_at(&mut self, buffer: &[u8]) -> anyhow::Result<()> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::WriteAt,
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

    /// Seek file to position.
    /// Returns the new position.
    pub fn seek(&mut self, pos: SeekFrom) -> anyhow::Result<u64> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::Seek { seek_from: pos},
        };
        let message = Request::new()
            .target(("our", "vfs", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&ipc)?;
                match response {
                    VfsResponse::SeekFrom(new_pos) => Ok(new_pos),
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
