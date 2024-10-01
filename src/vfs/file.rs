use super::{
    parse_response, vfs_request, FileMetadata, SeekFrom, VfsAction, VfsError, VfsResponse,
};
use crate::{get_blob, PackageId};

/// Vfs helper struct for a file.
/// Opening or creating a file will give you a `Result<File, VfsError>`.
/// You can call its impl functions to interact with it.
pub struct File {
    pub path: String,
    pub timeout: u64,
}

impl File {
    /// Create a new file-manager struct with the given path and timeout.
    pub fn new<T: Into<String>>(path: T, timeout: u64) -> Self {
        Self {
            path: path.into(),
            timeout,
        }
    }

    fn drop(&self) {
        vfs_request(&self.path, VfsAction::CloseFile)
            .send()
            .unwrap();
    }

    /// Reads the entire file, from start position.
    /// Returns a vector of bytes.
    pub fn read(&self) -> Result<Vec<u8>, VfsError> {
        let message = vfs_request(&self.path, VfsAction::Read)
            .send_and_await_response(self.timeout)
            .unwrap()
            .map_err(|e| VfsError::IOError {
                error: e.to_string(),
                path: self.path.clone(),
            })?;

        match parse_response(message.body())? {
            VfsResponse::Read => {
                let data = match get_blob() {
                    Some(bytes) => bytes.bytes,
                    None => {
                        return Err(VfsError::ParseError {
                            error: "no blob".to_string(),
                            path: self.path.clone(),
                        })
                    }
                };
                Ok(data)
            }
            VfsResponse::Err(e) => Err(e.into()),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }

    /// Reads the entire file, from start position, into buffer.
    /// Returns the amount of bytes read.
    pub fn read_into(&self, buffer: &mut [u8]) -> Result<usize, VfsError> {
        let message = vfs_request(&self.path, VfsAction::Read)
            .send_and_await_response(self.timeout)
            .unwrap()
            .map_err(|e| VfsError::IOError {
                error: e.to_string(),
                path: self.path.clone(),
            })?;

        match parse_response(message.body())? {
            VfsResponse::Read => {
                let data = get_blob().unwrap_or_default().bytes;
                let len = std::cmp::min(data.len(), buffer.len());
                buffer[..len].copy_from_slice(&data[..len]);
                Ok(len)
            }
            VfsResponse::Err(e) => Err(e.into()),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }

    /// Read into buffer from current cursor position
    /// Returns the amount of bytes read.
    pub fn read_at(&self, buffer: &mut [u8]) -> Result<usize, VfsError> {
        let length = buffer.len();

        let message = vfs_request(&self.path, VfsAction::ReadExact(length as u64))
            .send_and_await_response(self.timeout)
            .unwrap()
            .map_err(|e| VfsError::IOError {
                error: e.to_string(),
                path: self.path.clone(),
            })?;

        match parse_response(message.body())? {
            VfsResponse::Read => {
                let data = get_blob().unwrap_or_default().bytes;
                let len = std::cmp::min(data.len(), buffer.len());
                buffer[..len].copy_from_slice(&data[..len]);
                Ok(len)
            }
            VfsResponse::Err(e) => Err(e.into()),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }

    /// Reads until end of file from current cursor position
    /// Returns a vector of bytes.
    pub fn read_to_end(&self) -> Result<Vec<u8>, VfsError> {
        let message = vfs_request(&self.path, VfsAction::ReadToEnd)
            .send_and_await_response(self.timeout)
            .unwrap()
            .map_err(|e| VfsError::IOError {
                error: e.to_string(),
                path: self.path.clone(),
            })?;

        match parse_response(message.body())? {
            VfsResponse::Read => Ok(get_blob().unwrap_or_default().bytes),
            VfsResponse::Err(e) => Err(e),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }

    /// Reads until end of file from current cursor position, converts to String.
    /// Throws error if bytes aren't valid utf-8.
    /// Returns a vector of bytes.
    pub fn read_to_string(&self) -> Result<String, VfsError> {
        let message = vfs_request(&self.path, VfsAction::ReadToString)
            .send_and_await_response(self.timeout)
            .unwrap()
            .map_err(|e| VfsError::IOError {
                error: e.to_string(),
                path: self.path.clone(),
            })?;

        match parse_response(message.body())? {
            VfsResponse::ReadToString(s) => Ok(s),
            VfsResponse::Err(e) => Err(e),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }

    /// Write entire slice as the new file.
    /// Truncates anything that existed at path before.
    pub fn write(&self, buffer: &[u8]) -> Result<(), VfsError> {
        let message = vfs_request(&self.path, VfsAction::Write)
            .blob_bytes(buffer)
            .send_and_await_response(self.timeout)
            .unwrap()
            .map_err(|e| VfsError::IOError {
                error: e.to_string(),
                path: self.path.clone(),
            })?;

        match parse_response(message.body())? {
            VfsResponse::Ok => Ok(()),
            VfsResponse::Err(e) => Err(e),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }

    /// Write buffer to file at current position, overwriting any existing data.
    pub fn write_all(&mut self, buffer: &[u8]) -> Result<(), VfsError> {
        let message = vfs_request(&self.path, VfsAction::WriteAll)
            .blob_bytes(buffer)
            .send_and_await_response(self.timeout)
            .unwrap()
            .map_err(|e| VfsError::IOError {
                error: e.to_string(),
                path: self.path.clone(),
            })?;

        match parse_response(message.body())? {
            VfsResponse::Ok => Ok(()),
            VfsResponse::Err(e) => Err(e),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }

    /// Write buffer to the end position of file.
    pub fn append(&mut self, buffer: &[u8]) -> Result<(), VfsError> {
        let message = vfs_request(&self.path, VfsAction::Append)
            .blob_bytes(buffer)
            .send_and_await_response(self.timeout)
            .unwrap()
            .map_err(|e| VfsError::IOError {
                error: e.to_string(),
                path: self.path.clone(),
            })?;

        match parse_response(message.body())? {
            VfsResponse::Ok => Ok(()),
            VfsResponse::Err(e) => Err(e),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }

    /// Seek file to position.
    /// Returns the new position.
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, VfsError> {
        let message = vfs_request(&self.path, VfsAction::Seek { seek_from: pos })
            .send_and_await_response(self.timeout)
            .unwrap()
            .map_err(|e| VfsError::IOError {
                error: e.to_string(),
                path: self.path.clone(),
            })?;

        match parse_response(message.body())? {
            VfsResponse::SeekFrom(new_pos) => Ok(new_pos),
            VfsResponse::Err(e) => Err(e),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }

    /// Copies a file to path, returns a new File.
    pub fn copy(&mut self, path: &str) -> Result<File, VfsError> {
        let message = vfs_request(
            &self.path,
            VfsAction::CopyFile {
                new_path: path.to_string(),
            },
        )
        .send_and_await_response(self.timeout)
        .unwrap()
        .map_err(|e| VfsError::IOError {
            error: e.to_string(),
            path: self.path.clone(),
        })?;

        match parse_response(message.body())? {
            VfsResponse::Ok => Ok(File {
                path: path.to_string(),
                timeout: self.timeout,
            }),
            VfsResponse::Err(e) => Err(e),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }

    /// Set file length, if given size > underlying file, fills it with 0s.
    pub fn set_len(&mut self, size: u64) -> Result<(), VfsError> {
        let message = vfs_request(&self.path, VfsAction::SetLen(size))
            .send_and_await_response(self.timeout)
            .unwrap()
            .map_err(|e| VfsError::IOError {
                error: e.to_string(),
                path: self.path.clone(),
            })?;

        match parse_response(message.body())? {
            VfsResponse::Ok => Ok(()),
            VfsResponse::Err(e) => Err(e),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }

    /// Metadata of a path, returns file type and length.
    pub fn metadata(&self) -> Result<FileMetadata, VfsError> {
        let message = vfs_request(&self.path, VfsAction::Metadata)
            .send_and_await_response(self.timeout)
            .unwrap()
            .map_err(|e| VfsError::IOError {
                error: e.to_string(),
                path: self.path.clone(),
            })?;

        match parse_response(message.body())? {
            VfsResponse::Metadata(metadata) => Ok(metadata),
            VfsResponse::Err(e) => Err(e),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }

    /// Syncs path file buffers to disk.
    pub fn sync_all(&self) -> Result<(), VfsError> {
        let message = vfs_request(&self.path, VfsAction::SyncAll)
            .send_and_await_response(self.timeout)
            .unwrap()
            .map_err(|e| VfsError::IOError {
                error: e.to_string(),
                path: self.path.clone(),
            })?;

        match parse_response(message.body())? {
            VfsResponse::Ok => Ok(()),
            VfsResponse::Err(e) => Err(e),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }
}

/// Creates a drive with path "/package_id/drive", gives you read and write caps.
/// Will only work on the same package_id as you're calling it from, unless you
/// have root capabilities.
pub fn create_drive(
    package_id: PackageId,
    drive: &str,
    timeout: Option<u64>,
) -> Result<String, VfsError> {
    let timeout = timeout.unwrap_or(5);
    let path = format!("/{}/{}", package_id, drive);

    let message = vfs_request(&path, VfsAction::CreateDrive)
        .send_and_await_response(timeout)
        .unwrap()
        .map_err(|e| VfsError::IOError {
            error: e.to_string(),
            path: path.clone(),
        })?;

    match parse_response(message.body())? {
        VfsResponse::Ok => Ok(path),
        VfsResponse::Err(e) => Err(e),
        _ => Err(VfsError::ParseError {
            error: "unexpected response".to_string(),
            path,
        }),
    }
}

/// Opens a file at path, if no file at path, creates one if boolean create is true.
pub fn open_file(path: &str, create: bool, timeout: Option<u64>) -> Result<File, VfsError> {
    let timeout = timeout.unwrap_or(5);

    let message = vfs_request(path, VfsAction::OpenFile { create })
        .send_and_await_response(timeout)
        .unwrap()
        .map_err(|e| VfsError::IOError {
            error: e.to_string(),
            path: path.to_string(),
        })?;

    match parse_response(message.body())? {
        VfsResponse::Ok => Ok(File {
            path: path.to_string(),
            timeout,
        }),
        VfsResponse::Err(e) => Err(e),
        _ => Err(VfsError::ParseError {
            error: "unexpected response".to_string(),
            path: path.to_string(),
        }),
    }
}

/// Creates a file at path, if file found at path, truncates it to 0.
pub fn create_file(path: &str, timeout: Option<u64>) -> Result<File, VfsError> {
    let timeout = timeout.unwrap_or(5);

    let message = vfs_request(path, VfsAction::CreateFile)
        .send_and_await_response(timeout)
        .unwrap()
        .map_err(|e| VfsError::IOError {
            error: e.to_string(),
            path: path.to_string(),
        })?;

    match parse_response(message.body())? {
        VfsResponse::Ok => Ok(File {
            path: path.to_string(),
            timeout,
        }),
        VfsResponse::Err(e) => Err(e),
        _ => Err(VfsError::ParseError {
            error: "unexpected response".to_string(),
            path: path.to_string(),
        }),
    }
}

/// Removes a file at path, errors if path not found or path is not a file.
pub fn remove_file(path: &str, timeout: Option<u64>) -> Result<(), VfsError> {
    let timeout = timeout.unwrap_or(5);

    let message = vfs_request(path, VfsAction::RemoveFile)
        .send_and_await_response(timeout)
        .unwrap()
        .map_err(|e| VfsError::IOError {
            error: e.to_string(),
            path: path.to_string(),
        })?;

    match parse_response(message.body())? {
        VfsResponse::Ok => Ok(()),
        VfsResponse::Err(e) => Err(e.into()),
        _ => Err(VfsError::ParseError {
            error: "unexpected response".to_string(),
            path: path.to_string(),
        }),
    }
}
