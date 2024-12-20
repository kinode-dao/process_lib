use crate::{get_blob, Message, PackageId, Request};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::marker::PhantomData;
use thiserror::Error;

/// Actions are sent to a specific key value database, `db` is the name,
/// `package_id` is the [`PackageId`]. Capabilities are checked, you can access another process's
/// database if it has given you the [`crate::Capability`].
#[derive(Debug, Serialize, Deserialize)]
pub struct KvRequest {
    pub package_id: PackageId,
    pub db: String,
    pub action: KvAction,
}

/// IPC Action format, representing operations that can be performed on the key-value runtime module.
/// These actions are included in a KvRequest sent to the kv:distro:sys runtime module.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum KvAction {
    /// Opens an existing key-value database or creates a new one if it doesn't exist.
    Open,
    /// Permanently deletes the entire key-value database.
    RemoveDb,
    /// Sets a value for the specified key in the database.
    ///
    /// # Parameters
    /// * `key` - The key as a byte vector
    /// * `tx_id` - Optional transaction ID if this operation is part of a transaction
    Set { key: Vec<u8>, tx_id: Option<u64> },
    /// Deletes a key-value pair from the database.
    ///
    /// # Parameters
    /// * `key` - The key to delete as a byte vector
    /// * `tx_id` - Optional transaction ID if this operation is part of a transaction
    Delete { key: Vec<u8>, tx_id: Option<u64> },
    /// Retrieves the value associated with the specified key.
    ///
    /// # Parameters
    /// * `key` - The key to look up as a byte vector
    Get { key: Vec<u8> },
    /// Begins a new transaction for atomic operations.
    BeginTx,
    /// Commits all operations in the specified transaction.
    ///
    /// # Parameters
    /// * `tx_id` - The ID of the transaction to commit
    Commit { tx_id: u64 },
    /// Creates a backup of the database.
    Backup,
    /// Starts an iterator over the database contents.
    ///
    /// # Parameters
    /// * `prefix` - Optional byte vector to filter keys by prefix
    IterStart { prefix: Option<Vec<u8>> },
    /// Advances the iterator and returns the next batch of items.
    ///
    /// # Parameters
    /// * `iterator_id` - The ID of the iterator to advance
    /// * `count` - Maximum number of items to return
    IterNext { iterator_id: u64, count: u64 },
    /// Closes an active iterator.
    ///
    /// # Parameters
    /// * `iterator_id` - The ID of the iterator to close
    IterClose { iterator_id: u64 },
}

/// Response types for key-value store operations.
/// These responses are returned after processing a KvAction request.
#[derive(Debug, Serialize, Deserialize)]
pub enum KvResponse {
    /// Indicates successful completion of an operation.
    Ok,
    /// Returns the transaction ID for a newly created transaction.
    ///
    /// # Fields
    /// * `tx_id` - The ID of the newly created transaction
    BeginTx { tx_id: u64 },
    /// Returns the key that was retrieved from the database.
    ///
    /// # Fields
    /// * `key` - The retrieved key as a byte vector
    Get { key: Vec<u8> },
    /// Indicates an error occurred during the operation.
    ///
    /// # Fields
    /// * `error` - The specific error that occurred
    Err { error: KvError },
    /// Returns the ID of a newly created iterator.
    ///
    /// # Fields
    /// * `iterator_id` - The ID of the created iterator
    IterStart { iterator_id: u64 },
    /// Indicates whether the iterator has more items.
    ///
    /// # Fields
    /// * `done` - True if there are no more items to iterate over
    IterNext { done: bool },
    /// Confirms the closure of an iterator.
    ///
    /// # Fields
    /// * `iterator_id` - The ID of the closed iterator
    IterClose { iterator_id: u64 },
}

/// Errors that can occur during key-value store operations.
/// These errors are returned as part of `KvResponse::Err` when an operation fails.
#[derive(Debug, Serialize, Deserialize, Error)]
pub enum KvError {
    /// The requested database does not exist.
    #[error("Database does not exist")]
    NoDb,

    /// The requested key was not found in the database.
    #[error("Key not found in database")]
    KeyNotFound,

    /// No active transaction found for the given transaction ID.
    #[error("Transaction not found")]
    NoTx,

    /// The specified iterator was not found.
    #[error("Iterator not found")]
    NoIterator,

    /// The operation requires capabilities that the caller doesn't have.
    ///
    /// # Fields
    /// * `error` - Description of the missing capability or permission
    #[error("Missing required capability: {error}")]
    NoCap { error: String },

    /// An internal RocksDB error occurred during the operation.
    ///
    /// # Fields
    /// * `action` - The operation that was being performed
    /// * `error` - The specific error message from RocksDB
    #[error("RocksDB error during {action}: {error}")]
    RocksDBError { action: String, error: String },

    /// Error parsing or processing input data.
    ///
    /// # Fields
    /// * `error` - Description of what was invalid about the input
    #[error("Invalid input: {error}")]
    InputError { error: String },

    /// An I/O error occurred during the operation.
    ///
    /// # Fields
    /// * `error` - Description of the I/O error
    #[error("I/O error: {error}")]
    IOError { error: String },
}

/// Kv helper struct for a db.
/// Opening or creating a kv will give you a `Result<Kv>`.
/// You can call it's impl functions to interact with it.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
                action: KvAction::Get { key },
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
                    KvResponse::Err { error } => Err(error.into()),
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
                action: KvAction::Get { key },
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
                    KvResponse::Err { error } => Err(error.into()),
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
                    KvResponse::Err { error } => Err(error.into()),
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
                    KvResponse::Err { error } => Err(error.into()),
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
                    KvResponse::Err { error } => Err(error.into()),
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
            .send_and_await_response(self.timeout)?;

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

    /// Get all key-value pairs with an optional prefix
    ///
    /// # Example
    /// ```
    /// let entries = kv.iter_all(Some(&"user_"), 100)?;
    /// for (key, value) in entries {
    ///     println!("key: {}, value: {:?}", key, value);
    /// }
    /// ```
    pub fn iter_all(&self, prefix: Option<&K>, batch_size: u64) -> anyhow::Result<Vec<(K, V)>> {
        // Start the iterator
        let res = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::IterStart {
                    prefix: prefix.map(|p| serde_json::to_vec(p)).transpose()?,
                },
            })?)
            .send_and_await_response(self.timeout)?;

        let iterator_id = match res {
            Ok(Message::Response { body, .. }) => {
                match serde_json::from_slice::<KvResponse>(&body)? {
                    KvResponse::IterStart { iterator_id } => iterator_id,
                    KvResponse::Err { error } => return Err(error.into()),
                    _ => return Err(anyhow::anyhow!("kv: unexpected response")),
                }
            }
            _ => return Err(anyhow::anyhow!("kv: unexpected message")),
        };

        let mut all_entries = Vec::new();

        // Collect all entries
        loop {
            let res = Request::new()
                .target(("our", "kv", "distro", "sys"))
                .body(serde_json::to_vec(&KvRequest {
                    package_id: self.package_id.clone(),
                    db: self.db.clone(),
                    action: KvAction::IterNext {
                        iterator_id,
                        count: batch_size,
                    },
                })?)
                .send_and_await_response(self.timeout)?;

            match res {
                Ok(Message::Response { body, .. }) => {
                    match serde_json::from_slice::<KvResponse>(&body)? {
                        KvResponse::IterNext { done } => {
                            let blob = get_blob().ok_or_else(|| anyhow::anyhow!("No blob data"))?;
                            let entries: Vec<(Vec<u8>, Vec<u8>)> =
                                serde_json::from_slice(&blob.bytes)?;
                            for (key_bytes, value_bytes) in entries {
                                let key = serde_json::from_slice(&key_bytes)?;
                                let value = serde_json::from_slice(&value_bytes)?;
                                all_entries.push((key, value));
                            }
                            if done {
                                break;
                            }
                        }
                        KvResponse::Err { error } => return Err(error.into()),
                        _ => return Err(anyhow::anyhow!("kv: unexpected response")),
                    }
                }
                _ => return Err(anyhow::anyhow!("kv: unexpected message")),
            }
        }

        // Clean up
        let _ = Request::new()
            .target(("our", "kv", "distro", "sys"))
            .body(serde_json::to_vec(&KvRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: KvAction::IterClose { iterator_id },
            })?)
            .send_and_await_response(self.timeout)?;

        Ok(all_entries)
    }

    /// Get all keys with an optional prefix
    ///
    /// # Example
    /// ```
    /// let keys = kv.collect_keys(Some(&"user_"))?;
    /// for key in keys {
    ///     println!("key: {}", key);
    /// }
    /// ```
    pub fn collect_keys(&self, prefix: Option<&K>) -> anyhow::Result<Vec<K>> {
        Ok(self
            .iter_all(prefix, 100)?
            .into_iter()
            .map(|(k, _)| k)
            .collect())
    }

    /// Get all values with an optional key prefix
    ///
    /// # Example
    /// ```
    /// let values = kv.collect_values(Some(&"user_"))?;
    /// for value in values {
    ///     println!("value: {:?}", value);
    /// }
    /// ```
    pub fn collect_values(&self, prefix: Option<&K>) -> anyhow::Result<Vec<V>> {
        Ok(self
            .iter_all(prefix, 100)?
            .into_iter()
            .map(|(_, v)| v)
            .collect())
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
                    KvResponse::Err { error } => Err(error.into()),
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
                action: KvAction::Get { key: key.to_vec() },
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
                    KvResponse::Err { error } => Err(error.into()),
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
                    KvResponse::Err { error } => Err(error.into()),
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
                    KvResponse::Err { error } => Err(error.into()),
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
                KvResponse::Err { error } => Err(error.into()),
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
                KvResponse::Err { error } => Err(error.into()),
                _ => Err(anyhow::anyhow!("kv: unexpected response {:?}", response)),
            }
        }
        _ => Err(anyhow::anyhow!("kv: unexpected message: {:?}", res)),
    }
}
