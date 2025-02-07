use crate::{Message, Request as KiRequest};
pub use alloy::rpc::client::Authorization;
pub use alloy::rpc::json_rpc::ErrorPayload;
pub use alloy::rpc::types::eth::pubsub::SubscriptionResult;
pub use alloy::rpc::types::pubsub::Params;
pub use alloy::rpc::types::{
    request::{TransactionInput, TransactionRequest},
    Block, BlockId, BlockNumberOrTag, FeeHistory, Filter, FilterBlockOption, Log, Transaction,
    TransactionReceipt,
};
pub use alloy_primitives::{Address, BlockHash, BlockNumber, Bytes, TxHash, U128, U256, U64, U8};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

/// Subscription kind. Pulled directly from alloy (https://github.com/alloy-rs/alloy).
/// Why? Because alloy is not yet 1.0 and the types in this interface must be stable.
/// If alloy SubscriptionKind changes, we can implement a transition function in runtime
/// for this type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub enum SubscriptionKind {
    /// New block headers subscription.
    ///
    /// Fires a notification each time a new header is appended to the chain, including chain
    /// reorganizations. In case of a chain reorganization the subscription will emit all new
    /// headers for the new chain. Therefore the subscription can emit multiple headers on the same
    /// height.
    NewHeads,
    /// Logs subscription.
    ///
    /// Returns logs that are included in new imported blocks and match the given filter criteria.
    /// In case of a chain reorganization previous sent logs that are on the old chain will be
    /// resent with the removed property set to true. Logs from transactions that ended up in the
    /// new chain are emitted. Therefore, a subscription can emit logs for the same transaction
    /// multiple times.
    Logs,
    /// New Pending Transactions subscription.
    ///
    /// Returns the hash or full tx for all transactions that are added to the pending state and
    /// are signed with a key that is available in the node. When a transaction that was
    /// previously part of the canonical chain isn't part of the new canonical chain after a
    /// reorganization its again emitted.
    NewPendingTransactions,
    /// Node syncing status subscription.
    ///
    /// Indicates when the node starts or stops synchronizing. The result can either be a boolean
    /// indicating that the synchronization has started (true), finished (false) or an object with
    /// various progress indicators.
    Syncing,
}

/// The Action and Request type that can be made to eth:distro:sys. Any process with messaging
/// capabilities can send this action to the eth provider.
///
/// Will be serialized and deserialized using `serde_json::to_vec` and `serde_json::from_slice`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EthAction {
    /// Subscribe to logs with a custom filter. ID is to be used to unsubscribe.
    /// Logs come in as JSON value which can be parsed to [`alloy::rpc::types::eth::pubsub::SubscriptionResult`]
    SubscribeLogs {
        sub_id: u64,
        chain_id: u64,
        kind: SubscriptionKind,
        params: serde_json::Value,
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

/// Incoming [`crate::Request`] containing subscription updates or errors that processes will receive.
/// Can deserialize all incoming requests from eth:distro:sys to this type.
///
/// Will be serialized and deserialized using `serde_json::to_vec` and `serde_json::from_slice`.
pub type EthSubResult = Result<EthSub, EthSubError>;

/// Incoming type for successful subscription updates.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EthSub {
    pub id: u64,
    /// can be parsed to [`alloy::rpc::types::eth::pubsub::SubscriptionResult`]
    pub result: serde_json::Value,
}

/// If your subscription is closed unexpectedly, you will receive this.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EthSubError {
    pub id: u64,
    pub error: String,
}

/// The [`crate::Response`] body type which a process will get from requesting
/// with an [`EthAction`] will be of this type, serialized and deserialized
/// using [`serde_json::to_vec`] and [`serde_json::from_slice`].
///
/// In the case of an [`EthAction::SubscribeLogs`] request, the response will indicate if
/// the subscription was successfully created or not.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EthResponse {
    Ok,
    Response(serde_json::Value),
    Err(EthError),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EthError {
    /// RPC provider returned an error.
    /// Can be parsed to [`alloy::rpc::json_rpc::ErrorPayload`]
    RpcError(serde_json::Value),
    /// provider module cannot parse message
    MalformedRequest,
    /// No RPC provider for the chain
    NoRpcForChain,
    /// Subscription closed
    SubscriptionClosed(u64),
    /// Invalid method
    InvalidMethod(String),
    /// Invalid parameters
    InvalidParams,
    /// Permission denied
    PermissionDenied,
    /// RPC timed out
    RpcTimeout,
    /// RPC gave garbage back
    RpcMalformedResponse,
}

impl fmt::Display for EthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EthError::RpcError(e) => write!(f, "RPC error: {:?}", e),
            EthError::MalformedRequest => write!(f, "Malformed request"),
            EthError::NoRpcForChain => write!(f, "No RPC provider for chain"),
            EthError::SubscriptionClosed(id) => write!(f, "Subscription {} closed", id),
            EthError::InvalidMethod(m) => write!(f, "Invalid method: {}", m),
            EthError::InvalidParams => write!(f, "Invalid parameters"),
            EthError::PermissionDenied => write!(f, "Permission denied"),
            EthError::RpcTimeout => write!(f, "RPC request timed out"),
            EthError::RpcMalformedResponse => write!(f, "RPC returned malformed response"),
        }
    }
}

impl Error for EthError {}

/// The action type used for configuring eth:distro:sys. Only processes which have the "root"
/// [`crate::Capability`] from eth:distro:sys can successfully send this action.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    /// Get the list of current providers as a [`SavedConfigs`] object.
    GetProviders,
    /// Get the current access settings.
    GetAccessSettings,
    /// Get the state of calls and subscriptions. Used for debugging.
    GetState,
}

/// Response type from an [`EthConfigAction`] request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EthConfigResponse {
    Ok,
    /// Response from a GetProviders request.
    /// Note the [`crate::net::KnsUpdate`] will only have the correct `name` field.
    /// The rest of the Update is not saved in this module.
    Providers(SavedConfigs),
    /// Response from a GetAccessSettings request.
    AccessSettings(AccessSettings),
    /// Permission denied due to missing [`crate::Capability`]
    PermissionDenied,
    /// Response from a GetState request
    State {
        active_subscriptions: HashMap<crate::Address, HashMap<u64, Option<String>>>, // None if local, Some(node_provider_name) if remote
        outstanding_requests: HashSet<u64>,
    },
}

/// Settings for our ETH provider
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccessSettings {
    pub public: bool,           // whether or not other nodes can access through us
    pub allow: HashSet<String>, // whitelist for access (only used if public == false)
    pub deny: HashSet<String>,  // blacklist for access (always used)
}

pub type SavedConfigs = HashSet<ProviderConfig>;

/// Provider config. Can currently be a node or a ws provider instance.
#[derive(Clone, Debug, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub struct ProviderConfig {
    pub chain_id: u64,
    pub trusted: bool,
    pub provider: NodeOrRpcUrl,
}

#[derive(Clone, Debug, Serialize, Hash, Eq, PartialEq)]
pub enum NodeOrRpcUrl {
    Node {
        kns_update: crate::net::KnsUpdate,
        use_as_provider: bool, // false for just-routers inside saved config
    },
    RpcUrl {
        url: String,
        auth: Option<Authorization>,
    },
}

impl std::cmp::PartialEq<str> for NodeOrRpcUrl {
    fn eq(&self, other: &str) -> bool {
        match self {
            NodeOrRpcUrl::Node { kns_update, .. } => kns_update.name == other,
            NodeOrRpcUrl::RpcUrl { url, .. } => url == other,
        }
    }
}

impl<'de> Deserialize<'de> for NodeOrRpcUrl {
    fn deserialize<D>(serde::deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum RpcUrlHelper {
            String(String),
            Struct {
                url: String,
                auth: Option<Authorization>,
            },
        }

        #[derive(Deserialize)]
        #[serde(tag = "type")]
        enum Helper {
            Node {
                kns_update: crate::core::KnsUpdate,
                use_as_provider: bool,
            },
            RpcUrl(RpcUrlHelper),
        }

        let helper = Helper::deserialize(deserializer)?;

        Ok(match helper {
            Helper::Node {
                kns_update,
                use_as_provider,
            } => NodeOrRpcUrl::Node {
                kns_update,
                use_as_provider,
            },
            Helper::RpcUrl(url_helper) => match url_helper {
                RpcUrlHelper::String(url) => NodeOrRpcUrl::RpcUrl { url, auth: None },
                RpcUrlHelper::Struct { url, auth } => NodeOrRpcUrl::RpcUrl { url, auth },
            },
        })
    }
}

/// An EVM chain provider. Create this object to start making RPC calls.
/// Set the chain_id to determine which chain to call: requests will fail
/// unless the node this process is running on has access to a provider
/// for that chain.
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    /// Sends a request based on the specified [`EthAction`] and parses the response.
    ///
    /// This function constructs a request targeting the Ethereum distribution system, serializes the provided [`EthAction`],
    /// and sends it. It awaits a response with a specified timeout, then attempts to parse the response into the expected
    /// type `T`. This method is generic and can be used for various Ethereum actions by specifying the appropriate [`EthAction`]
    /// and return type `T`.
    pub fn send_request_and_parse_response<T: serde::de::DeserializeOwned>(
        &self,
        action: EthAction,
    ) -> Result<T, EthError> {
        let resp = KiRequest::new()
            .target(("our", "eth", "distro", "sys"))
            .body(serde_json::to_vec(&action).unwrap())
            .send_and_await_response(self.request_timeout)
            .unwrap()
            .map_err(|_| EthError::RpcTimeout)?;

        match resp {
            Message::Response { body, .. } => match serde_json::from_slice::<EthResponse>(&body) {
                Ok(EthResponse::Response(value)) => {
                    serde_json::from_value::<T>(value).map_err(|_| EthError::RpcMalformedResponse)
                }
                Ok(EthResponse::Err(e)) => Err(e),
                _ => Err(EthError::RpcMalformedResponse),
            },
            _ => Err(EthError::RpcMalformedResponse),
        }
    }

    /// Retrieves the current block number.
    ///
    /// # Returns
    /// A `Result<u64, EthError>` representing the current block number.
    pub fn get_block_number(&self) -> Result<u64, EthError> {
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
    /// A `Result<U256, EthError>` representing the balance of the address.
    pub fn get_balance(&self, address: Address, tag: Option<BlockId>) -> Result<U256, EthError> {
        let params = serde_json::to_value((
            address,
            tag.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)),
        ))
        .unwrap();
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
    /// A `Result<Vec<Log>, EthError>` containing the logs that match the filter.
    pub fn get_logs(&self, filter: &Filter) -> Result<Vec<Log>, EthError> {
        // NOTE: filter must be encased by a tuple to be serialized correctly
        let Ok(params) = serde_json::to_value((filter,)) else {
            return Err(EthError::InvalidParams);
        };
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_getLogs".to_string(),
            params,
        };

        self.send_request_and_parse_response::<Vec<Log>>(action)
    }

    /// Retrieves the current gas price.
    ///
    /// # Returns
    /// A `Result<U256, EthError>` representing the current gas price.
    pub fn get_gas_price(&self) -> Result<U256, EthError> {
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
    /// A `Result<U256, EthError>` representing the number of transactions sent from the address.
    pub fn get_transaction_count(
        &self,
        address: Address,
        tag: Option<BlockId>,
    ) -> Result<U256, EthError> {
        let Ok(params) = serde_json::to_value((address, tag.unwrap_or_default())) else {
            return Err(EthError::InvalidParams);
        };
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
    /// A `Result<Option<Block>, EthError>` representing the block, if found.
    pub fn get_block_by_hash(
        &self,
        hash: BlockHash,
        full_tx: bool,
    ) -> Result<Option<Block>, EthError> {
        let Ok(params) = serde_json::to_value((hash, full_tx)) else {
            return Err(EthError::InvalidParams);
        };
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
    /// A `Result<Option<Block>, EthError>` representing the block, if found.
    pub fn get_block_by_number(
        &self,
        number: BlockNumberOrTag,
        full_tx: bool,
    ) -> Result<Option<Block>, EthError> {
        let Ok(params) = serde_json::to_value((number, full_tx)) else {
            return Err(EthError::InvalidParams);
        };
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
    /// A `Result<Bytes, EthError>` representing the data stored at the given address and key.
    pub fn get_storage_at(
        &self,
        address: Address,
        key: U256,
        tag: Option<BlockId>,
    ) -> Result<Bytes, EthError> {
        let Ok(params) = serde_json::to_value((address, key, tag.unwrap_or_default())) else {
            return Err(EthError::InvalidParams);
        };
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
    /// A `Result<Bytes, EthError>` representing the code stored at the given address.
    pub fn get_code_at(&self, address: Address, tag: BlockId) -> Result<Bytes, EthError> {
        let Ok(params) = serde_json::to_value((address, tag)) else {
            return Err(EthError::InvalidParams);
        };
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
    /// A `Result<Option<Transaction>, EthError>` representing the transaction, if found.
    pub fn get_transaction_by_hash(&self, hash: TxHash) -> Result<Option<Transaction>, EthError> {
        // NOTE: hash must be encased by a tuple to be serialized correctly
        let Ok(params) = serde_json::to_value((hash,)) else {
            return Err(EthError::InvalidParams);
        };
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
    /// A `Result<Option<TransactionReceipt>, EthError>` representing the transaction receipt, if found.
    pub fn get_transaction_receipt(
        &self,
        hash: TxHash,
    ) -> Result<Option<TransactionReceipt>, EthError> {
        // NOTE: hash must be encased by a tuple to be serialized correctly
        let Ok(params) = serde_json::to_value((hash,)) else {
            return Err(EthError::InvalidParams);
        };
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
    /// A `Result<U256, EthError>` representing the estimated gas amount.
    pub fn estimate_gas(
        &self,
        tx: TransactionRequest,
        block: Option<BlockId>,
    ) -> Result<U256, EthError> {
        let Ok(params) = serde_json::to_value((tx, block.unwrap_or_default())) else {
            return Err(EthError::InvalidParams);
        };
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
    /// A `Result<Vec<Address>, EthError>` representing the list of accounts.
    /// Note: This function may return an empty list depending on the node's configuration and capabilities.
    pub fn get_accounts(&self) -> Result<Vec<Address>, EthError> {
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
    /// A `Result<FeeHistory, EthError>` representing the fee history for the specified range.
    pub fn get_fee_history(
        &self,
        block_count: U256,
        last_block: BlockNumberOrTag,
        reward_percentiles: Vec<f64>,
    ) -> Result<FeeHistory, EthError> {
        let Ok(params) = serde_json::to_value((block_count, last_block, reward_percentiles)) else {
            return Err(EthError::InvalidParams);
        };
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
    /// A `Result<Bytes, EthError>` representing the result of the call.
    pub fn call(&self, tx: TransactionRequest, block: Option<BlockId>) -> Result<Bytes, EthError> {
        let Ok(params) = serde_json::to_value((tx, block.unwrap_or_default())) else {
            return Err(EthError::InvalidParams);
        };
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_call".to_string(),
            params,
        };

        self.send_request_and_parse_response::<Bytes>(action)
    }

    /// Returns a Kimap instance with the default address using this provider.
    pub fn kimap(&self) -> crate::kimap::Kimap {
        crate::kimap::Kimap::default(self.request_timeout)
    }

    /// Returns a Kimap instance with a custom address using this provider.
    pub fn kimap_with_address(self, address: Address) -> crate::kimap::Kimap {
        crate::kimap::Kimap::new(self, address)
    }

    /// Sends a raw transaction to the network.
    ///
    /// # Parameters
    /// - `tx`: The raw transaction data.
    ///
    /// # Returns
    /// A `Result<TxHash, EthError>` representing the hash of the transaction once it has been sent.
    pub fn send_raw_transaction(&self, tx: Bytes) -> Result<TxHash, EthError> {
        let action = EthAction::Request {
            chain_id: self.chain_id,
            method: "eth_sendRawTransaction".to_string(),
            // NOTE: tx must be encased by a tuple to be serialized correctly
            params: serde_json::to_value((tx,)).unwrap(),
        };

        self.send_request_and_parse_response::<TxHash>(action)
    }

    /// Subscribes to logs without waiting for a confirmation.
    ///
    /// WARNING: some RPC providers will throw an error if a subscription filter
    /// has the `from_block` parameter. Make sure to avoid this parameter for subscriptions
    /// even while using it for getLogs.
    ///
    /// # Parameters
    /// - `sub_id`: The subscription ID to be used for unsubscribing.
    /// - `filter`: The filter criteria for the logs.
    ///
    /// # Returns
    /// A `Result<(), EthError>` indicating whether the subscription was created.
    pub fn subscribe(&self, sub_id: u64, filter: Filter) -> Result<(), EthError> {
        let action = EthAction::SubscribeLogs {
            sub_id,
            chain_id: self.chain_id,
            kind: SubscriptionKind::Logs,
            params: serde_json::to_value(Params::Logs(Box::new(filter)))
                .map_err(|_| EthError::InvalidParams)?,
        };

        let Ok(body) = serde_json::to_vec(&action) else {
            return Err(EthError::InvalidParams);
        };

        let resp = KiRequest::new()
            .target(("our", "eth", "distro", "sys"))
            .body(body)
            .send_and_await_response(self.request_timeout)
            .unwrap()
            .map_err(|_| EthError::RpcTimeout)?;

        match resp {
            Message::Response { body, .. } => {
                let response = serde_json::from_slice::<EthResponse>(&body);
                match response {
                    Ok(EthResponse::Ok) => Ok(()),
                    Ok(EthResponse::Err(e)) => Err(e),
                    _ => Err(EthError::RpcMalformedResponse),
                }
            }
            _ => Err(EthError::RpcMalformedResponse),
        }
    }

    /// Subscribe in a loop until successful
    pub fn subscribe_loop(
        &self,
        sub_id: u64,
        filter: Filter,
        print_verbosity_success: u8,
        print_verbosity_error: u8,
    ) {
        loop {
            match self.subscribe(sub_id, filter.clone()) {
                Ok(()) => break,
                Err(_) => {
                    crate::print_to_terminal(
                        print_verbosity_error,
                        "failed to subscribe to chain! trying again in 5s...",
                    );
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    continue;
                }
            }
        }
        crate::print_to_terminal(print_verbosity_success, "subscribed to logs successfully");
    }

    /// Unsubscribes from a previously created subscription.
    ///
    /// # Parameters
    /// - `sub_id`: The subscription ID to unsubscribe from.
    ///
    /// # Returns
    /// A `Result<(), EthError>` indicating whether the subscription was cancelled.
    pub fn unsubscribe(&self, sub_id: u64) -> Result<(), EthError> {
        let action = EthAction::UnsubscribeLogs(sub_id);

        let resp = KiRequest::new()
            .target(("our", "eth", "distro", "sys"))
            .body(serde_json::to_vec(&action).map_err(|_| EthError::MalformedRequest)?)
            .send_and_await_response(self.request_timeout)
            .unwrap()
            .map_err(|_| EthError::RpcTimeout)?;

        match resp {
            Message::Response { body, .. } => match serde_json::from_slice::<EthResponse>(&body) {
                Ok(EthResponse::Ok) => Ok(()),
                _ => Err(EthError::RpcMalformedResponse),
            },
            _ => Err(EthError::RpcMalformedResponse),
        }
    }
}
