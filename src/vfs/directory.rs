use super::{parse_response, vfs_request, DirEntry, VfsAction, VfsError, VfsResponse};

/// Vfs helper struct for a directory.
/// Opening or creating a directory will give you a Result<Directory>.
/// You can call it's impl functions to interact with it.
pub struct Directory {
    pub path: String,
    pub timeout: u64,
}

impl Directory {
    /// Iterates through children of directory, returning a vector of DirEntries.
    /// DirEntries contain the path and file type of each child.
    pub fn read(&self) -> Result<Vec<DirEntry>, VfsError> {
        let message = vfs_request(&self.path, VfsAction::ReadDir)
            .send_and_await_response(self.timeout)
            .unwrap()
            .map_err(|e| VfsError::IOError {
                error: e.to_string(),
                path: self.path.clone(),
            })?;

        match parse_response(message.body())? {
            VfsResponse::ReadDir(entries) => Ok(entries),
            VfsResponse::Err(e) => Err(e),
            _ => Err(VfsError::ParseError {
                error: "unexpected response".to_string(),
                path: self.path.clone(),
            }),
        }
    }
}

/// Opens or creates a directory at path.
/// If trying to create an existing directory, will just give you the path.
pub fn open_dir(path: &str, create: bool, timeout: Option<u64>) -> Result<Directory, VfsError> {
    let timeout = timeout.unwrap_or(5);
    if !create {
        return Ok(Directory {
            path: path.to_string(),
            timeout,
        });
    }

    let message = vfs_request(path, VfsAction::CreateDir)
        .send_and_await_response(timeout)
        .unwrap()
        .map_err(|e| VfsError::IOError {
            error: e.to_string(),
            path: path.to_string(),
        })?;

    match parse_response(message.body())? {
        VfsResponse::Ok => Ok(Directory {
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

/// Removes a dir at path, errors if path not found or path is not a directory.
pub fn remove_dir(path: &str, timeout: Option<u64>) -> Result<(), VfsError> {
    let timeout = timeout.unwrap_or(5);

    let message = vfs_request(path, VfsAction::RemoveDir)
        .send_and_await_response(timeout)
        .unwrap()
        .map_err(|e| VfsError::IOError {
            error: e.to_string(),
            path: path.to_string(),
        })?;

    match parse_response(message.body())? {
        VfsResponse::Ok => Ok(()),
        VfsResponse::Err(e) => Err(e),
        _ => Err(VfsError::ParseError {
            error: "unexpected response".to_string(),
            path: path.to_string(),
        }),
    }
}
