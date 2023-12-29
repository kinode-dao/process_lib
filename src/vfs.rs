use serde::{Deserialize, Serialize};
use crate::{get_payload, Message, PackageId, Request};

#[derive(Debug, Serialize, Deserialize)]
pub struct VfsRequest {
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

pub fn create_drive(package_id: PackageId, drive: String) -> anyhow::Result<()> {
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
                VfsResponse::Ok => Ok(()),
                VfsResponse::Err(e) => Err(anyhow::anyhow!("vfs: create drive error: {:?}", e)),
                _ => Err(anyhow::anyhow!("vfs: unexpected response")), 
            }
        }
        _ => return Err(anyhow::anyhow!("vfs: unexpected message")),
    }
}

pub async fn open_file(path: String, create: Option<bool>) -> anyhow::Result<File> {
    let action = match create {
        Some(true) => VfsAction::CreateFile,
        _ => VfsAction::OpenFile,
    };

    let request = VfsRequest {
        path: path.clone(),
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
                VfsResponse::Ok => Ok(File { path }),
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
    pub async fn read(&self, buffer: &mut [u8]) -> anyhow::Result<usize> {
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

    pub async fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::Write,
        };
        let message = Request::new()
            .target(("our", "vfs", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&request)?)
            .payload_bytes(data)
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

    pub async fn seek(&mut self, pos: SeekFrom) -> anyhow::Result<u64> {
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
    pub async fn set_len(&mut self, size: u64) -> anyhow::Result<()> {
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

    pub async fn metadata(&self) -> anyhow::Result<FileMetadata> {
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

    pub async fn sync_all(&self) -> anyhow::Result<()> {
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

