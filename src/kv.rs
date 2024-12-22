use crate::{get_blob, Message, PackageId, Request};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::marker::PhantomData;
use thiserror::Error;

/// Actions are sent to a specific key value database. `db` is the name,
/// `package_id` is the [`PackageId`] that created the database. Capabilities
/// are checked: you can access another process's database if it has given
/// you the read and/or write capability to do so.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KvRequest {
    pub package_id: PackageId,
    pub db: String,
    pub action: KvAction,
}

/// IPC Action format representing operations that can be performed on the
/// key-value runtime module. These actions are included in a [`KvRequest`]
/// sent to the `kv:distro:sys` runtime module.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum KvAction {
    /// Opens an existing key-value database or creates a new one if it doesn't exist.
    /// Requires `package_id` in [`KvRequest`] to match the package ID of the sender.
    /// The sender will own the database and can remove it with [`KvAction::RemoveDb`].
    ///
    /// A successful open will respond with [`KvResponse::Ok`]. Any error will be
    /// contained in the [`KvResponse::Err`] variant.
    Open,
    /// Permanently deletes the entire key-value database.
    /// Requires `package_id` in [`KvRequest`] to match the package ID of the sender.
    /// Only the owner can remove the database.
    ///
    /// A successful remove will respond with [`KvResponse::Ok`]. Any error will be
    /// contained in the [`KvResponse::Err`] variant.
    RemoveDb,
    /// Sets a value for the specified key in the database.
    ///
    /// # Parameters
    /// * `key` - The key as a byte vector
    /// * `tx_id` - Optional transaction ID if this operation is part of a transaction
    /// * blob: [`Vec<u8>`] - Byte vector to store for the key
    ///
    /// Using this action requires the sender to have the write capability
    /// for the database.
    ///
    /// A successful set will respond with [`KvResponse::Ok`]. Any error will be
    /// contained in the [`KvResponse::Err`] variant.
    Set { key: Vec<u8>, tx_id: Option<u64> },
    /// Deletes a key-value pair from the database.
    ///
    /// # Parameters
    /// * `key` - The key to delete as a byte vector
    /// * `tx_id` - Optional transaction ID if this operation is part of a transaction
    ///
    /// Using this action requires the sender to have the write capability
    /// for the database.
    ///
    /// A successful delete will respond with [`KvResponse::Ok`]. Any error will be
    /// contained in the [`KvResponse::Err`] variant.
    Delete { key: Vec<u8>, tx_id: Option<u64> },
    /// Retrieves the value associated with the specified key.
    ///
    /// # Parameters
    /// * The key to look up as a byte vector
    ///
    /// Using this action requires the sender to have the read capability
    /// for the database.
    ///
    /// A successful get will respond with [`KvResponse::Get`], where the response blob
    /// contains the value associated with the key if any. Any error will be
    /// contained in the [`KvResponse::Err`] variant.
    Get(Vec<u8>),
    /// Begins a new transaction for atomic operations.
    ///
    /// Sending this will prompt a [`KvResponse::BeginTx`] response with the
    /// transaction ID. Any error will be contained in the [`KvResponse::Err`] variant.
    BeginTx,
    /// Commits all operations in the specified transaction.
    ///
    /// # Parameters
    /// * `tx_id` - The ID of the transaction to commit
    ///
    /// A successful commit will respond with [`KvResponse::Ok`]. Any error will be
    /// contained in the [`KvResponse::Err`] variant.
    Commit { tx_id: u64 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum KvResponse {
    /// Indicates successful completion of an operation.
    /// Sent in response to actions Open, RemoveDb, Set, Delete, and Commit.
    Ok,
    /// Returns the transaction ID for a newly created transaction.
    ///
    /// # Fields
    /// * `tx_id` - The ID of the newly created transaction
    BeginTx { tx_id: u64 },
    /// Returns the value for the key that was retrieved from the database.
    ///
    /// # Parameters
    /// * The retrieved key as a byte vector
    /// * blob: [`Vec<u8>`] - Byte vector associated with the key
    Get(Vec<u8>),
    /// Indicates an error occurred during the operation.
    Err(KvError),
}

#[derive(Clone, Debug, Serialize, Deserialize, Error)]
pub enum KvError {
    #[error("db [{0}, {1}] does not exist")]
    NoDb(PackageId, String),
    #[error("key not found")]
    KeyNotFound,
    #[error("no transaction {0} found")]
    NoTx(u64),
    #[error("no write capability for requested DB")]
    NoWriteCap,
    #[error("no read capability for requested DB")]
    NoReadCap,
    #[error("request to open or remove DB with mismatching package ID")]
    MismatchingPackageId,
    #[error("failed to generate capability for new DB")]
    AddCapFailed,
    #[error("kv got a malformed request that either failed to deserialize or was missing a required blob")]
    MalformedRequest,
    #[error("RocksDB internal error: {0}")]
    RocksDBError(String),
    #[error("IO error: {0}")]
    IOError(String),
}

/// The JSON parameters contained in all capabilities issued by `kv:distro:sys`.
///
/// # Fields
/// * `kind` - The kind of capability, either [`KvCapabilityKind::Read`] or [`KvCapabilityKind::Write`]
/// * `db_key` - The database key, a tuple of the [`PackageId`] that created the database and the database name
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KvCapabilityParams {
    pub kind: KvCapabilityKind,
    pub db_key: (PackageId, String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KvCapabilityKind {
    Read,
    Write,
}

/// Kv helper struct for a db.
/// Opening or creating a kv will give you a `Result<Kv>`.
/// You can call it's impl functions to interact with it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Kv<K, V> {
    pub package_id: PackageId,
    pub db: String,
    pub timeout: u64,
    _marker: PhantomData<(K, V)>,
}

impl<K, V> Kv<K, V>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    /// Get a value.
    pub fn get(&self, key: &K) -> anyhow::Result<V> {
        let key = serde_json::to_vec(key)?;
        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Get(key),
            })?)
            .send_and_await_response(self.timeout)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::Get { .. } => {
                        let bytes = match get_blob() {
                            Some(bytes) => bytes.bytes,
                            None => return Err(anyhow::anyhow!("kv: no blob")),
                        };
                        let value = serde_json::from_slice::<V>(&bytes)
                            .map_err(|e| anyhow::anyhow!("Failed to deserialize value: {}", e))?;
                        Ok(value)
                    }
                    KvResponse::Err(error) => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    /// Get a value as a different type T
    pub fn get_as<T>(&self, key: &K) -> anyhow::Result<T>
    where
        T: DeserializeOwned,
    {
        let key = serde_json::to_vec(key)?;
        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Get(key),
            })?)
            .send_and_await_response(self.timeout)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::Get { .. } => {
                        let bytes = match get_blob() {
                            Some(bytes) => bytes.bytes,
                            None => return Err(anyhow::anyhow!("kv: no blob")),
                        };
                        let value = serde_json::from_slice::<T>(&bytes)
                            .map_err(|e| anyhow::anyhow!("Failed to deserialize value: {}", e))?;
                        Ok(value)
                    }
                    KvResponse::Err(error) => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    /// Set a value, optionally in a transaction.
    pub fn set(&self, key: &K, value: &V, tx_id: Option<u64>) -> anyhow::Result<()> {
        let key = serde_json::to_vec(key)?;
        let value = serde_json::to_vec(value)?;

        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Set { key, tx_id },
            })?)
            .blob_bytes(value)
            .send_and_await_response(self.timeout)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::Ok => Ok(()),
                    KvResponse::Err(error) => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    /// Set a value as a different type T
    pub fn set_as<T>(&self, key: &K, value: &T, tx_id: Option<u64>) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        let key = serde_json::to_vec(key)?;
        let value = serde_json::to_vec(value)?;

        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Set { key, tx_id },
            })?)
            .blob_bytes(value)
            .send_and_await_response(self.timeout)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::Ok => Ok(()),
                    KvResponse::Err(error) => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    /// Delete a value, optionally in a transaction.
    pub fn delete(&self, key: &K, tx_id: Option<u64>) -> anyhow::Result<()> {
        let key = serde_json::to_vec(key)?;
        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Delete { key, tx_id },
            })?)
            .send_and_await_response(self.timeout)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::Ok => Ok(()),
                    KvResponse::Err(error) => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    /// Delete a value with a different key type
    pub fn delete_as<T>(&self, key: &T, tx_id: Option<u64>) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        let key = serde_json::to_vec(key)?;

        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Delete { key, tx_id },
            })?)
            .send_and_await_response(self.timeout)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::Ok => Ok(()),
                    KvResponse::Err(error) => Err(error.into()),
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
            .send_and_await_response(self.timeout)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::BeginTx { tx_id } => Ok(tx_id),
                    KvResponse::Err(error) => Err(error.into()),
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
            .send_and_await_response(self.timeout)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::Ok => Ok(()),
                    KvResponse::Err(error) => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }
}

impl Kv<Vec<u8>, Vec<u8>> {
    /// Get raw bytes directly
    pub fn get_raw(&self, key: &[u8]) -> anyhow::Result<Vec<u8>> {
        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Get(key.to_vec()),
            })?)
            .send_and_await_response(self.timeout)?;

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
                    KvResponse::Err { 0: error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    /// Set raw bytes directly
    pub fn set_raw(&self, key: &[u8], value: &[u8], tx_id: Option<u64>) -> anyhow::Result<()> {
        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Set {
                    key: key.to_vec(),
                    tx_id,
                },
            })?)
            .blob_bytes(value.to_vec())
            .send_and_await_response(self.timeout)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::Ok => Ok(()),
                    KvResponse::Err { 0: error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }

    /// Delete raw bytes directly
    pub fn delete_raw(&self, key: &[u8], tx_id: Option<u64>) -> anyhow::Result<()> {
        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::Delete {
                    key: key.to_vec(),
                    tx_id,
                },
            })?)
            .send_and_await_response(self.timeout)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<KvResponse>(&body)?;

                match response {
                    KvResponse::Ok => Ok(()),
                    KvResponse::Err { 0: error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
        }
    }
}

/// Helper function to open a raw bytes key-value store
pub fn open_raw(
    package_id: PackageId,
    db: &str,
    timeout: Option<u64>,
) -> anyhow::Result<Kv<Vec<u8>, Vec<u8>>> {
    open(package_id, db, timeout)
}

/// Opens or creates a kv db.
pub fn open<K, V>(package_id: PackageId, db: &str, timeout: Option<u64>) -> anyhow::Result<Kv<K, V>>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    let timeout = timeout.unwrap_or(5);

    let res = Request::new()
        .target(("our", "kv", "distro", "sys"))
        .body(serde_json::to_vec(&KvRequest {
            package_id: package_id.clone(),
            db: db.to_string(),
            action: KvAction::Open,
        })?)
        .send_and_await_response(timeout)?;

    match res {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<KvResponse>(&body)?;

            match response {
                KvResponse::Ok => Ok(Kv {
                    package_id,
                    db: db.to_string(),
                    timeout,
                    _marker: PhantomData,
                }),
                KvResponse::Err(error) => Err(error.into()),
                _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
            }
        }
        _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
    }
}

/// Removes and deletes a kv db.
pub fn remove_db(package_id: PackageId, db: &str, timeout: Option<u64>) -> anyhow::Result<()> {
    let timeout = timeout.unwrap_or(5);

    let res = Request::new()
        .target(("our", "kv", "distro", "sys"))
        .body(serde_json::to_vec(&KvRequest {
            package_id: package_id.clone(),
            db: db.to_string(),
            action: KvAction::RemoveDb,
        })?)
        .send_and_await_response(timeout)?;

    match res {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<KvResponse>(&body)?;

            match response {
                KvResponse::Ok => Ok(()),
                KvResponse::Err(error) => Err(error.into()),
                _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
            }
        }
        _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
    }
}
