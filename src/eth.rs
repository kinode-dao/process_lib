use crate::{Message, Request as KiRequest};
pub use alloy_primitives::{Address, BlockHash, BlockNumber, Bytes, TxHash, U128, U256, U64, U8};
pub use alloy_rpc_types::pubsub::{Params, SubscriptionKind, SubscriptionResult};
pub use alloy_rpc_types::{
    request::{TransactionInput, TransactionRequest},
    Block, BlockId, BlockNumberOrTag, FeeHistory, Filter, FilterBlockOption, Log, Transaction,
    TransactionReceipt,
};
use serde::{Deserialize, Serialize};

/// The Action and Request type that can be made to eth:distro:sys. Any process with messaging
/// capabilities can send this action to the eth provider.
///
/// Will be serialized and deserialized using `serde_json::to_vec` and `serde_json::from_slice`.
#[derive(Debug, Serialize, Deserialize)]
pub enum EthAction {
    /// Subscribe to logs with a custom filter. ID is to be used to unsubscribe.
    /// Logs come in as alloy_rpc_types::pubsub::SubscriptionResults
    SubscribeLogs {
        sub_id: u64,
        chain_id: u64,
        kind: SubscriptionKind,
        params: Params,
    },
    /// Kill a SubscribeLogs subscription of a given ID, to stop getting updates.
    UnsubscribeLogs(u64),
    /// Raw request. Used by kinode_process_lib.
    Request {
        chain_id: u64,
        method: String,
        params: serde_json::Value,
    },
}

/// Incoming Result type for subscription updates or errors that processes will receive.
/// Can deserialize all incoming requests from eth:distro:sys to this type.
///
/// Will be serialized and deserialized using `serde_json::to_vec` and `serde_json::from_slice`.
pub type EthSubResult = Result<EthSub, EthSubError>;

/// Incoming Request type for successful subscription updates.
#[derive(Debug, Serialize, Deserialize)]
pub struct EthSub {
    pub id: u64,
    pub result: SubscriptionResult,
}

/// If your subscription is closed unexpectedly, you will receive this.
#[derive(Debug, Serialize, Deserialize)]
pub struct EthSubError {
    pub id: u64,
    pub error: String,
}

/// The Response type which a process will get from requesting with an [`EthAction`] will be
/// of the form `Result<(), EthError>`, serialized and deserialized using `serde_json::to_vec`
/// and `serde_json::from_slice`.
#[derive(Debug, Serialize, Deserialize)]
pub enum EthResponse {
    Ok,
    Response { value: serde_json::Value },
    Err(EthError),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EthError {
    /// No RPC provider for the chain
    NoRpcForChain,
    /// Underlying transport error
    TransportError(String),
    /// Subscription closed
    SubscriptionClosed(u64),
    /// The subscription ID was not found, so we couldn't unsubscribe.
    SubscriptionNotFound,
    /// Invalid method
    InvalidMethod(String),
    /// Permission denied
    PermissionDenied,
    /// Internal RPC error
    RpcError(String),
}

/// The action type used for configuring eth:distro:sys. Only processes which have the "root"
/// capability from eth:distro:sys can successfully send this action.
///
/// NOTE: changes to config will not be persisted between boots, they must be saved in .env
/// to be reflected between boots. TODO: can change this
#[derive(Debug, Serialize, Deserialize)]
pub enum EthConfigAction {
    /// Add a new provider to the list of providers.
    AddProvider(ProviderConfig),
    /// Remove a provider from the list of providers.
    /// The tuple is (chain_id, node_id/rpc_url).
    RemoveProvider((u64, String)),
    /// make our provider public
    SetPublic,
    /// make our provider not-public
    SetPrivate,
    /// add node to whitelist on a provider
    AllowNode(String),
    /// remove node from whitelist on a provider
    UnallowNode(String),
    /// add node to blacklist on a provider
    DenyNode(String),
    /// remove node from blacklist on a provider
    UndenyNode(String),
    /// Set the list of providers to a new list.
    /// Replaces all existing saved provider configs.
    SetProviders(SavedConfigs),
    /// Get the list of as a [`SavedConfigs`] object.
    GetProviders,
}

/// Response type from an [`EthConfigAction`] request.
#[derive(Debug, Serialize, Deserialize)]
pub enum EthConfigResponse {
    Ok,
    /// Response from a GetProviders request.
    Providers(SavedConfigs),
    /// Permission denied due to missing capability
    PermissionDenied,
}

pub type SavedConfigs = Vec<ProviderConfig>;

/// Provider config. Can currently be a node or a ws provider instance.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub chain_id: u64,
    pub trusted: bool,
    pub provider: NodeOrRpcUrl,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum NodeOrRpcUrl {
    Node(crate::kernel_types::KnsUpdate),
    RpcUrl(String),
}

/// An EVM chain provider. Create this object to start making RPC calls.
/// Set the chain_id to determine which chain to call: requests will fail
/// unless the node this process is running on has access to a provider
/// for that chain.
pub struct Provider {
    chain_id: u64,
    request_timeout: u64,
}

impl Provider {
    /// Instantiate a new provider.
    pub fn new(chain_id: u64, request_timeout: u64) -> Self {
        Self {
            chain_id,
            request_timeout,
        }
    }
    /// Sends a request based on the specified `EthAction` and parses the response.
    ///
    /// This function constructs a request targeting the Ethereum distribution system, serializes the provided `EthAction`,
    /// and sends it. It awaits a response with a specified timeout, then attempts to parse the response into the expected
    /// type `T`. This method is generic and can be used for various Ethereum actions by specifying the appropriate `EthAction`
    /// and return type `T`.
    pub fn send_request_and_parse_response<T: serde::de::DeserializeOwned>(
        &self,
        action: EthAction,
    ) -> anyhow::Result<T> {
        let resp = KiRequest::new()
            .target(("our", "eth", "distro", "sys"))
            .body(serde_json::to_vec(&action)?)
            .send_and_await_response(self.request_timeout)??;

        match resp {
            Message::Response { body, .. } => {
                let response = serde_json::from_slice::<EthResponse>(&body)?;
                match response {
                    EthResponse::Response { value } => serde_json::from_value::<T>(value)
                        .map_err(|e| anyhow::anyhow!("failed to parse response: {}", e)),
                    _ => Err(anyhow::anyhow!("unexpected response: {:?}", response)),
                }
            }
            _ => Err(anyhow::anyhow!("unexpected message type: {:?}", resp)),
        }
    }

    /// Retrieves the current block number.
    ///
    /// # Returns
    /// An `anyhow::Result<u64>` representing the current block number.
    pub fn get_block_number(&self) -> anyhow::Result<u64> {
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_blockNumber".to_string(),
            params: ().into(),
        };

        let res = self.send_request_and_parse_response::<U64>(action)?;
        Ok(res.to::<u64>())
    }

    /// Retrieves the balance of the given address at the specified block.
    ///
    /// # Parameters
    /// - `address`: The address to query the balance for.
    /// - `tag`: Optional block ID to specify the block at which the balance is queried.
    ///
    /// # Returns
    /// An `anyhow::Result<U256>` representing the balance of the address.
    pub fn get_balance(&self, address: Address, tag: Option<BlockId>) -> anyhow::Result<U256> {
        let params = serde_json::to_value((
            address,
            tag.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)),
        ))?;
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_getBalance".to_string(),
            params,
        };

        self.send_request_and_parse_response::<U256>(action)
    }

    /// Retrieves logs based on a filter.
    ///
    /// # Parameters
    /// - `filter`: The filter criteria for the logs.
    ///
    /// # Returns
    /// An `anyhow::Result<Vec<Log>>` containing the logs that match the filter.
    pub fn get_logs(&self, filter: &Filter) -> anyhow::Result<Vec<Log>> {
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_getLogs".to_string(),
            params: serde_json::to_value(filter)?,
        };

        self.send_request_and_parse_response::<Vec<Log>>(action)
    }

    /// Retrieves the current gas price.
    ///
    /// # Returns
    /// An `anyhow::Result<U256>` representing the current gas price.
    pub fn get_gas_price(&self) -> anyhow::Result<U256> {
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_gasPrice".to_string(),
            params: ().into(),
        };

        self.send_request_and_parse_response::<U256>(action)
    }

    /// Retrieves the number of transactions sent from the given address.
    ///
    /// # Parameters
    /// - `address`: The address to query the transaction count for.
    /// - `tag`: Optional block ID to specify the block at which the count is queried.
    ///
    /// # Returns
    /// An `anyhow::Result<U256>` representing the number of transactions sent from the address.
    pub fn get_transaction_count(
        &self,
        address: Address,
        tag: Option<BlockId>,
    ) -> anyhow::Result<U256> {
        let params = serde_json::to_value((address, tag.unwrap_or_default()))?;
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_getTransactionCount".to_string(),
            params,
        };

        self.send_request_and_parse_response::<U256>(action)
    }

    /// Retrieves a block by its hash.
    ///
    /// # Parameters
    /// - `hash`: The hash of the block to retrieve.
    /// - `full_tx`: Whether to return full transaction objects or just their hashes.
    ///
    /// # Returns
    /// An `anyhow::Result<Option<Block>>` representing the block, if found.
    pub fn get_block_by_hash(
        &self,
        hash: BlockHash,
        full_tx: bool,
    ) -> anyhow::Result<Option<Block>> {
        let params = serde_json::to_value((hash, full_tx))?;
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_getBlockByHash".to_string(),
            params,
        };

        self.send_request_and_parse_response::<Option<Block>>(action)
    }
    /// Retrieves a block by its number or tag.
    ///
    /// # Parameters
    /// - `number`: The number or tag of the block to retrieve.
    /// - `full_tx`: Whether to return full transaction objects or just their hashes.
    ///
    /// # Returns
    /// An `anyhow::Result<Option<Block>>` representing the block, if found.
    pub fn get_block_by_number(
        &self,
        number: BlockNumberOrTag,
        full_tx: bool,
    ) -> anyhow::Result<Option<Block>> {
        let params = serde_json::to_value((number, full_tx))?;
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_getBlockByNumber".to_string(),
            params,
        };

        self.send_request_and_parse_response::<Option<Block>>(action)
    }

    /// Retrieves the storage at a given address and key.
    ///
    /// # Parameters
    /// - `address`: The address of the storage to query.
    /// - `key`: The key of the storage slot to retrieve.
    /// - `tag`: Optional block ID to specify the block at which the storage is queried.
    ///
    /// # Returns
    /// An `anyhow::Result<Bytes>` representing the data stored at the given address and key.
    pub fn get_storage_at(
        &self,
        address: Address,
        key: U256,
        tag: Option<BlockId>,
    ) -> anyhow::Result<Bytes> {
        let params = serde_json::to_value((address, key, tag.unwrap_or_default()))?;
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_getStorageAt".to_string(),
            params,
        };

        self.send_request_and_parse_response::<Bytes>(action)
    }

    /// Retrieves the code at a given address.
    ///
    /// # Parameters
    /// - `address`: The address of the code to query.
    /// - `tag`: The block ID to specify the block at which the code is queried.
    ///
    /// # Returns
    /// An `anyhow::Result<Bytes>` representing the code stored at the given address.
    pub fn get_code_at(&self, address: Address, tag: BlockId) -> anyhow::Result<Bytes> {
        let params = serde_json::to_value((address, tag))?;
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_getCode".to_string(),
            params,
        };

        self.send_request_and_parse_response::<Bytes>(action)
    }

    /// Retrieves a transaction by its hash.
    ///
    /// # Parameters
    /// - `hash`: The hash of the transaction to retrieve.
    ///
    /// # Returns
    /// An `anyhow::Result<Option<Transaction>>` representing the transaction, if found.
    pub fn get_transaction_by_hash(&self, hash: TxHash) -> anyhow::Result<Option<Transaction>> {
        let params = serde_json::to_value(hash)?;
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_getTransactionByHash".to_string(),
            params,
        };

        self.send_request_and_parse_response::<Option<Transaction>>(action)
    }

    /// Retrieves the receipt of a transaction by its hash.
    ///
    /// # Parameters
    /// - `hash`: The hash of the transaction for which the receipt is requested.
    ///
    /// # Returns
    /// An `anyhow::Result<Option<TransactionReceipt>>` representing the transaction receipt, if found.
    pub fn get_transaction_receipt(
        &self,
        hash: TxHash,
    ) -> anyhow::Result<Option<TransactionReceipt>> {
        let params = serde_json::to_value(hash)?;
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_getTransactionReceipt".to_string(),
            params,
        };

        self.send_request_and_parse_response::<Option<TransactionReceipt>>(action)
    }

    /// Estimates the amount of gas that a transaction will consume.
    ///
    /// # Parameters
    /// - `tx`: The transaction request object containing the details of the transaction to estimate gas for.
    /// - `block`: Optional block ID to specify the block at which the gas estimate should be made.
    ///
    /// # Returns
    /// An `anyhow::Result<U256>` representing the estimated gas amount.
    pub fn estimate_gas(
        &self,
        tx: TransactionRequest,
        block: Option<BlockId>,
    ) -> anyhow::Result<U256> {
        let params = serde_json::to_value((tx, block.unwrap_or_default()))?;
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_estimateGas".to_string(),
            params,
        };

        self.send_request_and_parse_response::<U256>(action)
    }

    /// Retrieves the list of accounts controlled by the node.
    ///
    /// # Returns
    /// An `anyhow::Result<Vec<Address>>` representing the list of accounts.
    /// Note: This function may return an empty list depending on the node's configuration and capabilities.
    pub fn get_accounts(&self) -> anyhow::Result<Vec<Address>> {
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_accounts".to_string(),
            params: serde_json::Value::Array(vec![]),
        };

        self.send_request_and_parse_response::<Vec<Address>>(action)
    }

    /// Retrieves the fee history for a given range of blocks.
    ///
    /// # Parameters
    /// - `block_count`: The number of blocks to include in the history.
    /// - `last_block`: The ending block number or tag for the history range.
    /// - `reward_percentiles`: A list of percentiles to report fee rewards for.
    ///
    /// # Returns
    /// An `anyhow::Result<FeeHistory>` representing the fee history for the specified range.
    pub fn get_fee_history(
        &self,
        block_count: U256,
        last_block: BlockNumberOrTag,
        reward_percentiles: Vec<f64>,
    ) -> anyhow::Result<FeeHistory> {
        let params = serde_json::to_value((block_count, last_block, reward_percentiles))?;
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_feeHistory".to_string(),
            params,
        };

        self.send_request_and_parse_response::<FeeHistory>(action)
    }

    /// Executes a call transaction, which is directly executed in the VM of the node, but never mined into the blockchain.
    ///
    /// # Parameters
    /// - `tx`: The transaction request object containing the details of the call.
    /// - `block`: Optional block ID to specify the block at which the call should be made.
    ///
    /// # Returns
    /// An `anyhow::Result<Bytes>` representing the result of the call.
    pub fn call(&self, tx: TransactionRequest, block: Option<BlockId>) -> anyhow::Result<Bytes> {
        let params = serde_json::to_value((tx, block.unwrap_or_default()))?;
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_call".to_string(),
            params,
        };

        self.send_request_and_parse_response::<Bytes>(action)
    }

    /// Sends a raw transaction to the network.
    ///
    /// # Parameters
    /// - `tx`: The raw transaction data.
    ///
    /// # Returns
    /// An `anyhow::Result<TxHash>` representing the hash of the transaction once it has been sent.
    pub fn send_raw_transaction(&self, tx: Bytes) -> anyhow::Result<TxHash> {
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_sendRawTransaction".to_string(),
            params: serde_json::to_value(tx)?,
        };

        self.send_request_and_parse_response::<TxHash>(action)
    }

    /// Subscribes to logs without waiting for a confirmation.
    ///
    /// # Parameters
    /// - `sub_id`: The subscription ID to be used for unsubscribing.
    /// - `filter`: The filter criteria for the logs.
    ///
    /// # Returns
    /// An `anyhow::Result<()>` indicating whether the subscription was created.
    pub fn subscribe(&self, sub_id: u64, filter: Filter) -> anyhow::Result<()> {
        let action = EthAction::SubscribeLogs {
            sub_id,
            chain_id: self.chain_id,
            kind: SubscriptionKind::Logs,
            params: Params::Logs(Box::new(filter)),
        };

        let resp = KiRequest::new()
            .target(("our", "eth", "distro", "sys"))
            .body(serde_json::to_vec(&action)?)
            .send_and_await_response(self.request_timeout)??;

        match resp {
            Message::Response { body, .. } => {
                let response = serde_json::from_slice::<EthResponse>(&body)?;
                match response {
                    EthResponse::Ok => Ok(()),
                    EthResponse::Response { .. } => {
                        Err(anyhow::anyhow!("unexpected response: {:?}", response))
                    }
                    EthResponse::Err(e) => Err(anyhow::anyhow!("{e:?}")),
                }
            }
            _ => Err(anyhow::anyhow!("unexpected message type: {:?}", resp)),
        }
    }

    /// Unsubscribes from a previously created subscription.
    ///
    /// # Parameters
    /// - `sub_id`: The subscription ID to unsubscribe from.
    ///
    /// # Returns
    /// An `anyhow::Result<()>` indicating whether the subscription was cancelled.
    pub fn unsubscribe(&self, sub_id: u64) -> anyhow::Result<()> {
        let action = EthAction::UnsubscribeLogs(sub_id);

        let resp = KiRequest::new()
            .target(("our", "eth", "distro", "sys"))
            .body(serde_json::to_vec(&action)?)
            .send_and_await_response(self.request_timeout)??;

        match resp {
            Message::Response { body, .. } => {
                let response = serde_json::from_slice::<EthResponse>(&body)?;
                match response {
                    EthResponse::Ok => Ok(()),
                    EthResponse::Response { .. } => {
                        Err(anyhow::anyhow!("unexpected response: {:?}", response))
                    }
                    EthResponse::Err(e) => Err(anyhow::anyhow!("{e:?}")),
                }
            }
            _ => Err(anyhow::anyhow!("unexpected message type: {:?}", resp)),
        }
    }
}
