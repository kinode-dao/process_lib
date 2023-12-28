use crate::{get_payload, Message, PackageId, Request, Response};
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
    /// New is called to create a new database and be given capabilities,
    /// or to open a connection to an existing one.
    New,
    Set {
        key: Vec<u8>,
        tx_id: Option<u64>,
    },
    Delete {
        key: Vec<u8>,
        tx_id: Option<u64>,
    },
    Get {
        key: Vec<u8>,
    },
    BeginTx,
    Commit {
        tx_id: u64,
    },
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

pub fn new(package_id: PackageId, db: String) -> anyhow::Result<()> {
    let res = Request::new()
        .target(("our", "kv", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&KvRequest {
            package_id,
            db,
            action: KvAction::New,
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { ipc, .. }) => {
            let resp =
                serde_json::from_slice::<KvResponse>(&ipc).map_err(|e| KvError::InputError {
                    error: format!("kv: gave unparsable response: {}", e),
                })?;

            if let KvResponse::Ok = resp {
                Ok(())
            } else {
                Err(anyhow::anyhow!("kv: unexpected response: {:?}", resp))
            }
        }
        _ => return Err(anyhow::anyhow!("kv: unexpected response")),
    }
}

pub fn get(package_id: PackageId, db: String, key: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let res = Request::new()
        .target(("our", "kv", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&KvRequest {
            package_id,
            db,
            action: KvAction::Get { key },
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { ipc, .. }) => {
            let resp =
                serde_json::from_slice::<KvResponse>(&ipc).map_err(|e| KvError::InputError {
                    error: format!("kv: gave unparsable response: {}", e),
                })?;

            if let KvResponse::Get { .. } = resp {
                let bytes = match get_payload() {
                    Some(bytes) => bytes.bytes,
                    None => return Err(anyhow::anyhow!("kv: no payload")),
                };
                Ok(bytes)
            } else {
                Err(anyhow::anyhow!("kv: unexpected response: {:?}", resp))
            }
        }
        _ => return Err(anyhow::anyhow!("kv: unexpected response")),
    }
}

pub fn set(
    package_id: PackageId,
    db: String,
    key: Vec<u8>,
    value: Vec<u8>,
    tx_id: Option<u64>,
) -> anyhow::Result<()> {
    let res = Request::new()
        .target(("our", "kv", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&KvRequest {
            package_id,
            db,
            action: KvAction::Set { key, tx_id },
        })?)
        .payload_bytes(value)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { ipc, .. }) => {
            let resp =
                serde_json::from_slice::<KvResponse>(&ipc).map_err(|e| KvError::InputError {
                    error: format!("kv: gave unparsable response: {}", e),
                })?;

            if let KvResponse::Ok = resp {
                Ok(())
            } else {
                Err(anyhow::anyhow!("kv: unexpected response: {:?}", resp))
            }
        }
        _ => return Err(anyhow::anyhow!("kv: unexpected response")),
    }
}

pub fn delete(
    package_id: PackageId,
    db: String,
    key: Vec<u8>,
    tx_id: Option<u64>,
) -> anyhow::Result<()> {
    let res = Request::new()
        .target(("our", "kv", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&KvRequest {
            package_id,
            db,
            action: KvAction::Delete { key, tx_id },
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { ipc, .. }) => {
            let resp =
                serde_json::from_slice::<KvResponse>(&ipc).map_err(|e| KvError::InputError {
                    error: format!("kv: gave unparsable response: {}", e),
                })?;

            if let KvResponse::Ok = resp {
                Ok(())
            } else {
                Err(anyhow::anyhow!("kv: unexpected response: {:?}", resp))
            }
        }
        _ => return Err(anyhow::anyhow!("kv: unexpected response")),
    }
}

pub fn begin_tx(package_id: PackageId, db: String) -> anyhow::Result<u64> {
    let res = Request::new()
        .target(("our", "kv", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&KvRequest {
            package_id,
            db,
            action: KvAction::BeginTx,
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { ipc, .. }) => {
            let resp =
                serde_json::from_slice::<KvResponse>(&ipc).map_err(|e| KvError::InputError {
                    error: format!("kv: gave unparsable response: {}", e),
                })?;

            if let KvResponse::BeginTx { tx_id } = resp {
                Ok(tx_id)
            } else {
                Err(anyhow::anyhow!("kv: unexpected response: {:?}", resp))
            }
        }
        _ => return Err(anyhow::anyhow!("kv: unexpected response")),
    }
}

pub fn commit_tx(package_id: PackageId, db: String, tx_id: u64) -> anyhow::Result<()> {
    let res = Request::new()
        .target(("our", "kv", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&KvRequest {
            package_id,
            db,
            action: KvAction::Commit { tx_id },
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { ipc, .. }) => {
            let resp =
                serde_json::from_slice::<KvResponse>(&ipc).map_err(|e| KvError::InputError {
                    error: format!("kv: gave unparsable response: {}", e),
                })?;

            if let KvResponse::Ok = resp {
                Ok(())
            } else {
                Err(anyhow::anyhow!("kv: unexpected response: {:?}", resp))
            }
        }
        _ => return Err(anyhow::anyhow!("kv: unexpected response")),
    }
}
