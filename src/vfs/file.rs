use super::{FileMetadata, SeekFrom, VfsAction, VfsRequest, VfsResponse};
use crate::{get_blob, Message, PackageId, Request};

/// Vfs helper struct for a file.
/// Opening or creating a file will give you a Result<File>.
/// You can call it's impl functions to interact with it.
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
            .target(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&body)?;
                match response {
                    VfsResponse::Read => {
                        let data = match get_blob() {
                            Some(bytes) => bytes.bytes,
                            None => return Err(anyhow::anyhow!("vfs: no read blob")),
                        };
                        Ok(data)
                    }
                    VfsResponse::Err(e) => Err(e.into()),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
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
            .target(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&body)?;
                match response {
                    VfsResponse::Read => {
                        let data = match get_blob() {
                            Some(bytes) => bytes.bytes,
                            None => return Err(anyhow::anyhow!("vfs: no read blob")),
                        };
                        let len = std::cmp::min(data.len(), buffer.len());
                        buffer[..len].copy_from_slice(&data[..len]);
                        Ok(len)
                    }
                    VfsResponse::Err(e) => Err(e.into()),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
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
            .target(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&body)?;
                match response {
                    VfsResponse::Read => {
                        let data = match get_blob() {
                            Some(bytes) => bytes.bytes,
                            None => return Err(anyhow::anyhow!("vfs: no read blob")),
                        };
                        let len = std::cmp::min(data.len(), buffer.len());
                        buffer[..len].copy_from_slice(&data[..len]);
                        Ok(len)
                    }
                    VfsResponse::Err(e) => Err(e.into()),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
        }
    }

    /// Reads until end of file from current cursor position
    /// Returns a vector of bytes.
    pub fn read_to_end(&self) -> anyhow::Result<Vec<u8>> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::ReadToEnd,
        };
        let message = Request::new()
            .target(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&body)?;
                match response {
                    VfsResponse::Read => {
                        let data = match get_blob() {
                            Some(bytes) => bytes.bytes,
                            None => return Err(anyhow::anyhow!("vfs: no read blob")),
                        };
                        Ok(data)
                    }
                    VfsResponse::Err(e) => Err(e.into()),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
        }
    }

    /// Reads until end of file from current cursor position, converts to String.
    /// Throws error if bytes aren't valid utf-8.
    /// Returns a vector of bytes.
    pub fn read_to_string(&self) -> anyhow::Result<String> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::ReadToString,
        };
        let message = Request::new()
            .target(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&body)?;
                match response {
                    VfsResponse::ReadToString(s) => Ok(s),
                    VfsResponse::Err(e) => Err(e.into()),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
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
            .target(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&request)?)
            .blob_bytes(buffer)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&body)?;
                match response {
                    VfsResponse::Ok => Ok(()),
                    VfsResponse::Err(e) => Err(e.into()),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
        }
    }

    /// Write buffer to file at current position, overwriting any existing data.
    pub fn write_all(&mut self, buffer: &[u8]) -> anyhow::Result<()> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::WriteAll,
        };
        let message = Request::new()
            .target(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&request)?)
            .blob_bytes(buffer)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&body)?;
                match response {
                    VfsResponse::Ok => Ok(()),
                    VfsResponse::Err(e) => Err(e.into()),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
        }
    }

    /// Write buffer to the end position of file.
    pub fn append(&mut self, buffer: &[u8]) -> anyhow::Result<()> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::Append,
        };
        let message = Request::new()
            .target(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&request)?)
            .blob_bytes(buffer)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&body)?;
                match response {
                    VfsResponse::Ok => Ok(()),
                    VfsResponse::Err(e) => Err(e.into()),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
        }
    }

    /// Seek file to position.
    /// Returns the new position.
    pub fn seek(&mut self, pos: SeekFrom) -> anyhow::Result<u64> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::Seek { seek_from: pos },
        };
        let message = Request::new()
            .target(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&body)?;
                match response {
                    VfsResponse::SeekFrom(new_pos) => Ok(new_pos),
                    VfsResponse::Err(e) => Err(e.into()),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
        }
    }

    /// Copies a file to path, returns a new File.
    pub fn copy(&mut self, path: &str) -> anyhow::Result<File> {
        let request = VfsRequest {
            path: self.path.to_string(),
            action: VfsAction::CopyFile {
                new_path: path.to_string(),
            },
        };

        let message = Request::new()
            .target(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&body)?;
                match response {
                    VfsResponse::Ok => Ok(File {
                        path: path.to_string(),
                    }),
                    VfsResponse::Err(e) => Err(e.into()),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
        }
    }

    /// Set file length, if given size > underlying file, fills it with 0s.
    pub fn set_len(&mut self, size: u64) -> anyhow::Result<()> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::SetLen(size),
        };
        let message = Request::new()
            .target(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&body)?;
                match response {
                    VfsResponse::Ok => Ok(()),
                    VfsResponse::Err(e) => Err(e.into()),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
        }
    }

    /// Metadata of a path, returns file type and length.
    pub fn metadata(&self) -> anyhow::Result<FileMetadata> {
        let request = VfsRequest {
            path: self.path.clone(),
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

    /// Syncs path file buffers to disk.
    pub fn sync_all(&self) -> anyhow::Result<()> {
        let request = VfsRequest {
            path: self.path.clone(),
            action: VfsAction::SyncAll,
        };
        let message = Request::new()
            .target(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&body)?;
                match response {
                    VfsResponse::Ok => Ok(()),
                    VfsResponse::Err(e) => Err(e.into()),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
        }
    }
}

/// Creates a drive with path "/package_id/drive", gives you read and write caps.
/// Will only work on the same package_id as you're calling it from, unless you
/// have root capabilities.
pub fn create_drive(package_id: PackageId, drive: &str) -> anyhow::Result<String> {
    let path = format!("/{}/{}", package_id, drive);
    let res = Request::new()
        .target(("our", "vfs", "distro", "sys"))
        .body(serde_json::to_vec(&VfsRequest {
            path: path.clone(),
            action: VfsAction::CreateDrive,
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<VfsResponse>(&body)?;
            match response {
                VfsResponse::Ok => Ok(path),
                VfsResponse::Err(e) => Err(e.into()),
                _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
            }
        }
        _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", res)),
    }
}

/// Opens a file at path, if no file at path, creates one if boolean create is true.
pub fn open_file(path: &str, create: bool) -> anyhow::Result<File> {
    let request = VfsRequest {
        path: path.to_string(),
        action: VfsAction::OpenFile { create },
    };

    let message = Request::new()
        .target(("our", "vfs", "distro", "sys"))
        .body(serde_json::to_vec(&request)?)
        .send_and_await_response(5)?;

    match message {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<VfsResponse>(&body)?;
            match response {
                VfsResponse::Ok => Ok(File {
                    path: path.to_string(),
                }),
                VfsResponse::Err(e) => Err(e.into()),
                _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
            }
        }
        _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
    }
}

/// Creates a file at path, if file found at path, truncates it to 0.
pub fn create_file(path: &str) -> anyhow::Result<File> {
    let request = VfsRequest {
        path: path.to_string(),
        action: VfsAction::CreateFile,
    };

    let message = Request::new()
        .target(("our", "vfs", "distro", "sys"))
        .body(serde_json::to_vec(&request)?)
        .send_and_await_response(5)?;

    match message {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<VfsResponse>(&body)?;
            match response {
                VfsResponse::Ok => Ok(File {
                    path: path.to_string(),
                }),
                VfsResponse::Err(e) => Err(e.into()),
                _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
            }
        }
        _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
    }
}

/// Removes a file at path, errors if path not found or path is not a file.
pub fn remove_file(path: &str) -> anyhow::Result<()> {
    let request = VfsRequest {
        path: path.to_string(),
        action: VfsAction::RemoveFile,
    };

    let message = Request::new()
        .target(("our", "vfs", "distro", "sys"))
        .body(serde_json::to_vec(&request)?)
        .send_and_await_response(5)?;

    match message {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<VfsResponse>(&body)?;
            match response {
                VfsResponse::Ok => Ok(()),
                VfsResponse::Err(e) => Err(e.into()),
                _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
            }
        }
        _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
    }
}
