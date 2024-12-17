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
    IterStart { prefix: Option<Vec<u8>> },
    IterNext { iterator_id: u64, count: u64 },
    IterClose { iterator_id: u64 },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum KvResponse {
    Ok,
    BeginTx { tx_id: u64 },
    Get { key: Vec<u8> },
    Err { error: KvError },
    IterStart { iterator_id: u64 },
    IterNext { done: bool },
    IterClose { iterator_id: u64 },
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
                            let entries_bytes =
                                get_blob().ok_or_else(|| anyhow::anyhow!("No blob data"))?;
                            let entries: Vec<(Vec<u8>, Vec<u8>)> =
                                serde_json::from_slice(&entries_bytes)?;
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
