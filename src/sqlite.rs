use crate::{get_payload, Message, PackageId, Request};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct SqliteRequest {
    pub package_id: PackageId,
    pub db: String,
    pub action: SqliteAction,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SqliteAction {
    Open,
    RemoveDb,
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

pub struct Sqlite {
    pub package_id: PackageId,
    pub db: String,
}

pub fn open(package_id: PackageId, db: &str) -> anyhow::Result<Sqlite> {
    let res = Request::new()
        .target(("our", "sqlite", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&SqliteRequest {
            package_id: package_id.clone(),
            db: db.to_string(),
            action: SqliteAction::Open,
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { ipc, .. }) => {
            let response = serde_json::from_slice::<SqliteResponse>(&ipc)?;

            match response {
                SqliteResponse::Ok => Ok(Sqlite {
                    package_id,
                    db: db.to_string(),
                }),
                SqliteResponse::Err { error } => Err(error.into()),
                _ => Err(anyhow::anyhow!(
                    "sqlite: unexpected response {:?}",
                    response
                )),
            }
        }
        _ => return Err(anyhow::anyhow!("sqlite: unexpected message: {:?}", res)),
    }
}

pub fn remove_db(package_id: PackageId, db: &str) -> anyhow::Result<()> {
    let res = Request::new()
        .target(("our", "sqlite", "sys", "uqbar"))
        .ipc(serde_json::to_vec(&SqliteRequest {
            package_id: package_id.clone(),
            db: db.to_string(),
            action: SqliteAction::RemoveDb,
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { ipc, .. }) => {
            let response = serde_json::from_slice::<SqliteResponse>(&ipc)?;

            match response {
                SqliteResponse::Ok => Ok(()),
                SqliteResponse::Err { error } => Err(error.into()),
                _ => Err(anyhow::anyhow!(
                    "sqlite: unexpected response {:?}",
                    response
                )),
            }
        }
        _ => return Err(anyhow::anyhow!("sqlite: unexpected message: {:?}", res)),
    }
}

impl Sqlite {
    pub fn read(
        &self,
        query: String,
        params: Vec<SqlValue>,
    ) -> anyhow::Result<Vec<HashMap<String, serde_json::Value>>> {
        let res = Request::new()
            .target(("our", "sqlite", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&SqliteRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: SqliteAction::Read { query },
            })?)
            .payload_bytes(serde_json::to_vec(&params)?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<SqliteResponse>(&ipc)?;

                match response {
                    SqliteResponse::Read => {
                        let payload = get_payload().ok_or_else(|| SqliteError::InputError {
                            error: format!("sqlite: no payload"),
                        })?;
                        let values = serde_json::from_slice::<
                            Vec<HashMap<String, serde_json::Value>>,
                        >(&payload.bytes)
                        .map_err(|e| SqliteError::InputError {
                            error: format!("sqlite: gave unparsable response: {}", e),
                        })?;
                        Ok(values)
                    }
                    SqliteResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!(
                        "sqlite: unexpected response {:?}",
                        response
                    )),
                }
            }
            _ => return Err(anyhow::anyhow!("sqlite: unexpected message: {:?}", res)),
        }
    }

    pub fn write(
        &self,
        statement: String,
        params: Vec<serde_json::Value>,
        tx_id: Option<u64>,
    ) -> anyhow::Result<()> {
        let res = Request::new()
            .target(("our", "sqlite", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&SqliteRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: SqliteAction::Write { statement, tx_id },
            })?)
            .payload_bytes(serde_json::to_vec(&params)?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<SqliteResponse>(&ipc)?;

                match response {
                    SqliteResponse::Ok => Ok(()),
                    SqliteResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!(
                        "sqlite: unexpected response {:?}",
                        response
                    )),
                }
            }
            _ => return Err(anyhow::anyhow!("sqlite: unexpected message: {:?}", res)),
        }
    }

    pub fn begin_tx(&self) -> anyhow::Result<u64> {
        let res = Request::new()
            .target(("our", "sqlite", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&SqliteRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: SqliteAction::BeginTx,
            })?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<SqliteResponse>(&ipc)?;

                match response {
                    SqliteResponse::BeginTx { tx_id } => Ok(tx_id),
                    SqliteResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!(
                        "sqlite: unexpected response {:?}",
                        response
                    )),
                }
            }
            _ => return Err(anyhow::anyhow!("sqlite: unexpected message: {:?}", res)),
        }
    }

    pub fn commit_tx(&self, tx_id: u64) -> anyhow::Result<()> {
        let res = Request::new()
            .target(("our", "sqlite", "sys", "uqbar"))
            .ipc(serde_json::to_vec(&SqliteRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: SqliteAction::Commit { tx_id },
            })?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { ipc, .. }) => {
                let response = serde_json::from_slice::<SqliteResponse>(&ipc)?;

                match response {
                    SqliteResponse::Ok => Ok(()),
                    SqliteResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!(
                        "sqlite: unexpected response {:?}",
                        response
                    )),
                }
            }
            _ => return Err(anyhow::anyhow!("sqlite: unexpected message: {:?}", res)),
        }
    }
}
