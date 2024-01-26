use crate::{get_blob, Message, PackageId, Request};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Actions are sent to a specific key value database, "db" is the name,
/// "package_id" is the package. Capabilities are checked, you can access another process's
/// database if it has given you the capability.
#[derive(Debug, Serialize, Deserialize)]
pub struct KvRequest {
    pub package_id: PackageId,
    pub db: String,
    pub action: KvAction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum KvAction {
    Open,
    RemoveDb,
    Set { key: Vec<u8>, tx_id: Option<u64> },
    Delete { key: Vec<u8>, tx_id: Option<u64> },
    Get { key: Vec<u8> },
    BeginTx,
    Commit { tx_id: u64 },
    Backup,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum KvResponse {
    Ok,
    BeginTx { tx_id: u64 },
    Get { key: Vec<u8> },
    Err { error: KvError },
}

#[derive(Debug, Serialize, Deserialize, Error)]
pub enum KvError {
    #[error("kv: DbDoesNotExist")]
    NoDb,
    #[error("kv: KeyNotFound")]
    KeyNotFound,
    #[error("kv: no Tx found")]
    NoTx,
    #[error("kv: No capability: {error}")]
    NoCap { error: String },
    #[error("kv: rocksdb internal error: {error}")]
    RocksDBError { action: String, error: String },
    #[error("kv: input bytes/json/key error: {error}")]
    InputError { error: String },
    #[error("kv: IO error: {error}")]
    IOError { error: String },
}

/// Kv helper struct for a db.
/// Opening or creating a kv will give you a Result<Kv>.
/// You can call it's impl functions to interact with it.
#[derive(Debug, Serialize, Deserialize)]
pub struct Kv {
    pub package_id: PackageId,
    pub db: String,
}

impl Kv {
    /// Get a value.
    pub fn get(&self, key: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Get { key },
            })?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::Get { .. } => {
                        let bytes = match get_blob() {
                            Some(bytes) => bytes.bytes,
                            None => return Err(anyhow::anyhow!("kv: no blob")),
                        };
                        Ok(bytes)
                    }
                    KvResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    /// Set a value, optionally in a transaction.
    pub fn set(&self, key: Vec<u8>, value: Vec<u8>, tx_id: Option<u64>) -> anyhow::Result<()> {
        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Set { key, tx_id },
            })?)
            .blob_bytes(value)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::Ok => Ok(()),
                    KvResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    /// Delete a value, optionally in a transaction.
    pub fn delete(&self, key: Vec<u8>, tx_id: Option<u64>) -> anyhow::Result<()> {
        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Delete { key, tx_id },
            })?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::Ok => Ok(()),
                    KvResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    /// Begin a transaction.
    pub fn begin_tx(&self) -> anyhow::Result<u64> {
        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::BeginTx,
            })?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::BeginTx { tx_id } => Ok(tx_id),
                    KvResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    /// Commit a transaction.
    pub fn commit_tx(&self, tx_id: u64) -> anyhow::Result<()> {
        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Commit { tx_id },
            })?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::Ok => Ok(()),
                    KvResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }
}

/// Opens or creates a kv db.
pub fn open(package_id: PackageId, db: &str) -> anyhow::Result<Kv> {
    let res = Request::new()
        .target(("our", "kv", "distro", "sys"))
        .body(serde_json::to_vec(&KvRequest {
            package_id: package_id.clone(),
            db: db.to_string(),
            action: KvAction::Open,
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<KvResponse>(&body)?;

            match response {
                KvResponse::Ok => Ok(Kv {
                    package_id,
                    db: db.to_string(),
                }),
                KvResponse::Err { error } => Err(error.into()),
                _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
            }
        }
        _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
    }
}

/// Removes and deletes a kv db.
pub fn remove_db(package_id: PackageId, db: &str) -> anyhow::Result<()> {
    let res = Request::new()
        .target(("our", "kv", "distro", "sys"))
        .body(serde_json::to_vec(&KvRequest {
            package_id: package_id.clone(),
            db: db.to_string(),
            action: KvAction::RemoveDb,
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<KvResponse>(&body)?;

            match response {
                KvResponse::Ok => Ok(()),
                KvResponse::Err { error } => Err(error.into()),
                _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
            }
        }
        _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
    }
}
