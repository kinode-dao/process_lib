use super::{DirEntry, VfsAction, VfsRequest, VfsResponse};
use crate::{Message, Request};

/// Vfs helper struct for a directory.
/// Opening or creating a directory will give you a Result<Directory>.
/// You can call it's impl functions to interact with it.
pub struct Directory {
    pub path: String,
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
            .target(("our", "vfs", "distro", "sys"))
            .body(serde_json::to_vec(&request)?)
            .send_and_await_response(5)?;

        match message {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<VfsResponse>(&body)?;
                match response {
                    VfsResponse::ReadDir(entries) => Ok(entries),
                    VfsResponse::Err(e) => Err(e.into()),
                    _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
        }
    }
}

/// Opens or creates a directory at path.
/// If trying to create an existing directory, will just give you the path.
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
        .target(("our", "vfs", "distro", "sys"))
        .body(serde_json::to_vec(&request)?)
        .send_and_await_response(5)?;

    match message {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<VfsResponse>(&body)?;
            match response {
                VfsResponse::Ok => Ok(Directory {
                    path: path.to_string(),
                }),
                VfsResponse::Err(e) => Err(e.into()),
                _ => Err(anyhow::anyhow!("vfs: unexpected response: {:?}", response)),
            }
        }
        _ => Err(anyhow::anyhow!("vfs: unexpected message: {:?}", message)),
    }
}

/// Removes a dir at path, errors if path not found or path is not a directory.
pub fn remove_dir(path: &str) -> anyhow::Result<()> {
    let request = VfsRequest {
        path: path.to_string(),
        action: VfsAction::RemoveDir,
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
