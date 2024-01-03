use crate::{get_payload, Message, PackageId, Request};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct KvRequest {
    pub package_id: PackageId,
    pub db: String,
    pub action: KvAction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum KvAction {
    New,
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
    #[error("kv: DbAlreadyExists")]
    DbAlreadyExists,
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
pub struct Kv {
    pub package_id: PackageId,
    pub db: String,
}

pub fn new(package_id: PackageId, db: String) -> anyhow::Result<Kv> {
    let res = Request::new()
        .target(("our", "kv", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&KvRequest {
            package_id: package_id.clone(),
            db: db.clone(),
            action: KvAction::New,
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { ipc, .. }) => {
            let response = serde_json::from_slice::<KvResponse>(&ipc)?;

            match response {
                KvResponse::Ok => Ok(Kv { package_id, db }),
                KvResponse::Err { error } => Err(error.into()),
                _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
            }
        }
        _ => return Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
    }
}

impl Kv {
    pub fn get(&self, key: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let res = Request::new()
            .target(("our", "kv", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Get { key },
            })?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&ipc)?;

                match response {
                    KvResponse::Get { .. } => {
                        let bytes = match get_payload() {
                            Some(bytes) => bytes.bytes,
                            None => return Err(anyhow::anyhow!("kv: no payload")),
                        };
                        Ok(bytes)
                    }
                    KvResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => return Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    pub fn set(&self, key: Vec<u8>, value: Vec<u8>, tx_id: Option<u64>) -> anyhow::Result<()> {
        let res = Request::new()
            .target(("our", "kv", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Set { key, tx_id },
            })?)
            .payload_bytes(value)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&ipc)?;

                match response {
                    KvResponse::Ok => Ok(()),
                    KvResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => return Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    pub fn delete(&self, key: Vec<u8>, tx_id: Option<u64>) -> anyhow::Result<()> {
        let res = Request::new()
            .target(("our", "kv", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Delete { key, tx_id },
            })?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&ipc)?;

                match response {
                    KvResponse::Ok => Ok(()),
                    KvResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => return Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    pub fn begin_tx(&self) -> anyhow::Result<u64> {
        let res = Request::new()
            .target(("our", "kv", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::BeginTx,
            })?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&ipc)?;

                match response {
                    KvResponse::BeginTx { tx_id } => Ok(tx_id),
                    KvResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => return Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    pub fn commit_tx(&self, tx_id: u64) -> anyhow::Result<()> {
        let res = Request::new()
            .target(("our", "kv", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Commit { tx_id },
            })?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&ipc)?;

                match response {
                    KvResponse::Ok => Ok(()),
                    KvResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => return Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }
}
