use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::{PackageId, Request, Message, get_payload};

#[derive(Debug, Serialize, Deserialize)]
pub struct SqliteRequest {
    pub package_id: PackageId,
    pub db: String, 
    pub action: SqliteAction,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SqliteAction {
    New,
    Write {
        statement: String,
        tx_id: Option<u64>,
    },
    Read {
        query: String,
    },
    BeginTx,
    Commit {
        tx_id: u64,
    },
    Backup,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SqliteResponse {
    Ok,
    Read,
    BeginTx { tx_id: u64 },
    Err { error: SqliteError },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SqlValue {
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
    Boolean(bool),
    Null,
}

#[derive(Debug, Serialize, Deserialize, Error)]
pub enum SqliteError {
    #[error("sqlite: DbDoesNotExist")]
    NoDb,
    #[error("sqlite: DbAlreadyExists")]
    DbAlreadyExists,
    #[error("sqlite: NoTx")]
    NoTx,
    #[error("sqlite: No capability: {error}")]
    NoCap { error: String },
    #[error("sqlite: UnexpectedResponse")]
    UnexpectedResponse,
    #[error("sqlite: NotAWriteKeyword")]
    NotAWriteKeyword,
    #[error("sqlite: NotAReadKeyword")]
    NotAReadKeyword,
    #[error("sqlite: Invalid Parameters")]
    InvalidParameters,
    #[error("sqlite: IO error: {error}")]
    IOError { error: String },
    #[error("sqlite: rusqlite error: {error}")]
    RusqliteError { error: String },
    #[error("sqlite: input bytes/json/key error: {error}")]
    InputError { error: String },
}

pub fn new(
    package_id: PackageId,
    db: String,
) -> anyhow::Result<()> {
    let res = Request::new()
        .target(("our", "sqlite", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&SqliteRequest {
            package_id,
            db,
            action: SqliteAction::New,
        })?)
        .send_and_await_response(5)?;
    
    match res {
        Ok(Message::Response { ipc, .. }) => {
            let resp = serde_json::from_slice::<SqliteResponse>(&ipc).map_err(|e| SqliteError::InputError {
                error: format!("sqlite: gave unparsable response: {}", e),
            })?;

            if let SqliteResponse::Ok = resp {
                Ok(())
            } else {
                Err(anyhow::anyhow!("sqlite: unexpected response: {:?}", resp))
            }
        },
        _ => return Err(anyhow::anyhow!("sqlite: unexpected response")),
    }
}

pub fn read(
    package_id: PackageId,
    db: String,
    query: String,
    params: Vec<SqlValue>,
) -> anyhow::Result<Vec<HashMap<String, serde_json::Value>>> {
    let res = Request::new()
        .target(("our", "sqlite", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&SqliteRequest {
            package_id,
            db,
            action: SqliteAction::Read { query },
        })?)
        .payload_bytes(serde_json::to_vec(&params)?)
        .send_and_await_response(5)?;
    
    match res {
        Ok(Message::Response { ipc, .. }) => {
            let resp = serde_json::from_slice::<SqliteResponse>(&ipc).map_err(|e| SqliteError::InputError {
                error: format!("sqlite: gave unparsable response: {}", e),
            })?;

            if let SqliteResponse::Read = resp {
                let payload = get_payload().ok_or_else(|| SqliteError::InputError {
                    error: format!("sqlite: no payload"),
                })?;
                let values = serde_json::from_slice::<Vec<HashMap<String, serde_json::Value>>>(&payload.bytes).map_err(|e| SqliteError::InputError {
                    error: format!("sqlite: gave unparsable response: {}", e),
                })?;
                Ok(values)
            } else {
                Err(anyhow::anyhow!("sqlite: unexpected response: {:?}", resp))
            }
        },
        _ => return Err(anyhow::anyhow!("sqlite: unexpected response")),
    }
}

pub fn write(
    package_id: PackageId,
    db: String,
    statement: String,
    params: Vec<serde_json::Value>,
    tx_id: Option<u64>,
) -> anyhow::Result<()> {
    let res = Request::new()
        .target(("our", "sqlite", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&SqliteRequest {
            package_id,
            db,
            action: SqliteAction::Write { statement, tx_id },
        })?)
        .payload_bytes(serde_json::to_vec(&params)?)
        .send_and_await_response(5)?;
    
    match res {
        Ok(Message::Response { ipc, .. }) => {
            let resp = serde_json::from_slice::<SqliteResponse>(&ipc).map_err(|e| SqliteError::InputError {
                error: format!("sqlite: gave unparsable response: {}", e),
            })?;

            if let SqliteResponse::Ok = resp {
                Ok(())
            } else {
                Err(anyhow::anyhow!("sqlite: unexpected response: {:?}", resp))
            }
        },
        _ => return Err(anyhow::anyhow!("sqlite: unexpected response")),
    }
}

pub fn begin_tx(
    package_id: PackageId,
    db: String,
) -> anyhow::Result<u64> {
    let res = Request::new()
        .target(("our", "sqlite", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&SqliteRequest {
            package_id,
            db,
            action: SqliteAction::BeginTx,
        })?)
        .send_and_await_response(5)?;
    
    match res {
        Ok(Message::Response { ipc, .. }) => {
            let resp = serde_json::from_slice::<SqliteResponse>(&ipc).map_err(|e| SqliteError::InputError {
                error: format!("sqlite: gave unparsable response: {}", e),
            })?;

            if let SqliteResponse::BeginTx { tx_id } = resp {
                Ok(tx_id)
            } else {
                Err(anyhow::anyhow!("sqlite: unexpected response: {:?}", resp))
            }
        },
        _ => return Err(anyhow::anyhow!("sqlite: unexpected response")),
    }
}

pub fn commit_tx(
    package_id: PackageId,
    db: String,
    tx_id: u64,
) -> anyhow::Result<()> {
    let res = Request::new()
        .target(("our", "sqlite", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&SqliteRequest {
            package_id,
            db,
            action: SqliteAction::Commit { tx_id },
        })?)
        .send_and_await_response(5)?;
    
    match res {
        Ok(Message::Response { ipc, .. }) => {
            let resp = serde_json::from_slice::<SqliteResponse>(&ipc).map_err(|e| SqliteError::InputError {
                error: format!("sqlite: gave unparsable response: {}", e),
            })?;

            if let SqliteResponse::Ok = resp {
                Ok(())
            } else {
                Err(anyhow::anyhow!("sqlite: unexpected response: {:?}", resp))
            }
        },
        _ => return Err(anyhow::anyhow!("sqlite: unexpected response")),
    }
}