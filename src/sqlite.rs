use crate::{get_blob, Message, PackageId, Request};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Actions are sent to a specific sqlite database, "db" is the name,
/// "package_id" is the package. Capabilities are checked, you can access another process's
/// database if it has given you the capability.
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

/// Sqlite helper struct for a db.
/// Opening or creating a db will give you a Result<sqlite>.
/// You can call it's impl functions to interact with it.
pub struct Sqlite {
    pub package_id: PackageId,
    pub db: String,
}

impl Sqlite {
    /// Query database. Only allows sqlite read keywords.
    pub fn read(
        &self,
        query: String,
        params: Vec<serde_json::Value>,
    ) -> anyhow::Result<Vec<HashMap<String, serde_json::Value>>> {
        let res = Request::new()
            .target(("our", "sqlite", "distro", "sys"))
            .body(serde_json::to_vec(&SqliteRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: SqliteAction::Read { query },
            })?)
            .blob_bytes(serde_json::to_vec(&params)?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<SqliteResponse>(&body)?;

                match response {
                    SqliteResponse::Read => {
                        let blob = get_blob().ok_or_else(|| SqliteError::InputError {
                            error: "sqlite: no blob".to_string(),
                        })?;
                        let values = serde_json::from_slice::<
                            Vec<HashMap<String, serde_json::Value>>,
                        >(&blob.bytes)
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
            _ => Err(anyhow::anyhow!("sqlite: unexpected message: {:?}", res)),
        }
    }

    /// Execute a statement. Only allows sqlite write keywords.
    pub fn write(
        &self,
        statement: String,
        params: Vec<serde_json::Value>,
        tx_id: Option<u64>,
    ) -> anyhow::Result<()> {
        let res = Request::new()
            .target(("our", "sqlite", "distro", "sys"))
            .body(serde_json::to_vec(&SqliteRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: SqliteAction::Write { statement, tx_id },
            })?)
            .blob_bytes(serde_json::to_vec(&params)?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<SqliteResponse>(&body)?;

                match response {
                    SqliteResponse::Ok => Ok(()),
                    SqliteResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!(
                        "sqlite: unexpected response {:?}",
                        response
                    )),
                }
            }
            _ => Err(anyhow::anyhow!("sqlite: unexpected message: {:?}", res)),
        }
    }

    /// Begin a transaction.
    pub fn begin_tx(&self) -> anyhow::Result<u64> {
        let res = Request::new()
            .target(("our", "sqlite", "distro", "sys"))
            .body(serde_json::to_vec(&SqliteRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: SqliteAction::BeginTx,
            })?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<SqliteResponse>(&body)?;

                match response {
                    SqliteResponse::BeginTx { tx_id } => Ok(tx_id),
                    SqliteResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!(
                        "sqlite: unexpected response {:?}",
                        response
                    )),
                }
            }
            _ => Err(anyhow::anyhow!("sqlite: unexpected message: {:?}", res)),
        }
    }

    /// Commit a transaction.
    pub fn commit_tx(&self, tx_id: u64) -> anyhow::Result<()> {
        let res = Request::new()
            .target(("our", "sqlite", "distro", "sys"))
            .body(serde_json::to_vec(&SqliteRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: SqliteAction::Commit { tx_id },
            })?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<SqliteResponse>(&body)?;

                match response {
                    SqliteResponse::Ok => Ok(()),
                    SqliteResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!(
                        "sqlite: unexpected response {:?}",
                        response
                    )),
                }
            }
            _ => Err(anyhow::anyhow!("sqlite: unexpected message: {:?}", res)),
        }
    }
}

/// Open or create sqlite database.
pub fn open(package_id: PackageId, db: &str) -> anyhow::Result<Sqlite> {
    let res = Request::new()
        .target(("our", "sqlite", "distro", "sys"))
        .body(serde_json::to_vec(&SqliteRequest {
            package_id: package_id.clone(),
            db: db.to_string(),
            action: SqliteAction::Open,
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<SqliteResponse>(&body)?;

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
        _ => Err(anyhow::anyhow!("sqlite: unexpected message: {:?}", res)),
    }
}

/// Remove and delete sqlite database.
pub fn remove_db(package_id: PackageId, db: &str) -> anyhow::Result<()> {
    let res = Request::new()
        .target(("our", "sqlite", "distro", "sys"))
        .body(serde_json::to_vec(&SqliteRequest {
            package_id: package_id.clone(),
            db: db.to_string(),
            action: SqliteAction::RemoveDb,
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<SqliteResponse>(&body)?;

            match response {
                SqliteResponse::Ok => Ok(()),
                SqliteResponse::Err { error } => Err(error.into()),
                _ => Err(anyhow::anyhow!(
                    "sqlite: unexpected response {:?}",
                    response
                )),
            }
        }
        _ => Err(anyhow::anyhow!("sqlite: unexpected message: {:?}", res)),
    }
}
