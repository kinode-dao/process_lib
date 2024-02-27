use crate::{Message, Request as KiRequest};
pub use alloy_primitives::{Address, BlockHash, BlockNumber, Bytes, TxHash, U128, U256, U64, U8};
pub use alloy_rpc_types::pubsub::{Params, SubscriptionKind, SubscriptionResult};
pub use alloy_rpc_types::{
    request::{TransactionInput, TransactionRequest},
    Block, BlockId, BlockNumberOrTag, FeeHistory, Filter, FilterBlockOption, Log, Transaction,
    TransactionReceipt,
};
use serde::{Deserialize, Serialize};

/// The Action and Request type that can be made to eth:distro:sys.
/// Will be serialized and deserialized using `serde_json::to_vec` and `serde_json::from_slice`.
#[derive(Debug, Serialize, Deserialize)]
pub enum EthAction {
    /// Subscribe to logs with a custom filter. ID is to be used to unsubscribe.
    /// Logs come in as alloy_rpc_types::pubsub::SubscriptionResults
    SubscribeLogs {
        sub_id: u64,
        kind: SubscriptionKind,
        params: Params,
    },
    /// Kill a SubscribeLogs subscription of a given ID, to stop getting updates.
    UnsubscribeLogs(u64),
    /// Raw request. Used by kinode_process_lib.
    Request {
        method: String,
        params: serde_json::Value,
    },
}
/// Incoming subscription update.
#[derive(Debug, Serialize, Deserialize)]
pub struct EthSub {
    pub id: u64,
    pub result: SubscriptionResult,
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
    /// Underlying transport error
    TransportError(String),
    /// Subscription closed
    SubscriptionClosed(u64),
    /// The subscription ID was not found, so we couldn't unsubscribe.
    SubscriptionNotFound,
    /// Invalid method
    InvalidMethod(String),
    /// Permission denied
    PermissionDenied(String),
    /// Internal RPC error
    RpcError(String),
}

/// Sends a request based on the specified `EthAction` and parses the response.
///
/// This function constructs a request targeting the Ethereum distribution system, serializes the provided `EthAction`,
/// and sends it. It awaits a response with a specified timeout, then attempts to parse the response into the expected
/// type `T`. This method is generic and can be used for various Ethereum actions by specifying the appropriate `EthAction`
/// and return type `T`.
/// Note the timeout of 5s.
pub fn send_request_and_parse_response<T: serde::de::DeserializeOwned>(
    action: EthAction,
) -> anyhow::Result<T> {
    let resp = KiRequest::new()
        .target(("our", "eth", "distro", "sys"))
        .body(serde_json::to_vec(&action)?)
        .send_and_await_response(10)??;

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
pub fn get_block_number() -> anyhow::Result<u64> {
    let action = EthAction::Request {
        method: "eth_blockNumber".to_string(),
        params: ().into(),
    };

    let res = send_request_and_parse_response::<U64>(action)?;
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
pub fn get_balance(address: Address, tag: Option<BlockId>) -> anyhow::Result<U256> {
    let params = serde_json::to_value((
        address,
        tag.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)),
    ))?;
    let action = EthAction::Request {
        method: "eth_getBalance".to_string(),
        params,
    };

    send_request_and_parse_response::<U256>(action)
}

/// Retrieves logs based on a filter.
///
/// # Parameters
/// - `filter`: The filter criteria for the logs.
///
/// # Returns
/// An `anyhow::Result<Vec<Log>>` containing the logs that match the filter.
pub fn get_logs(filter: &Filter) -> anyhow::Result<Vec<Log>> {
    let action = EthAction::Request {
        method: "eth_getLogs".to_string(),
        params: serde_json::to_value((filter,))?,
    };

    send_request_and_parse_response::<Vec<Log>>(action)
}

/// Retrieves the current gas price.
///
/// # Returns
/// An `anyhow::Result<U256>` representing the current gas price.
pub fn get_gas_price() -> anyhow::Result<U256> {
    let action = EthAction::Request {
        method: "eth_gasPrice".to_string(),
        params: ().into(),
    };

    send_request_and_parse_response::<U256>(action)
}

/// Retrieves the chain ID.
///
/// # Returns
/// An `anyhow::Result<U256>` representing the chain ID.
pub fn get_chain_id() -> anyhow::Result<U256> {
    let action = EthAction::Request {
        method: "eth_chainId".to_string(),
        params: ().into(),
    };

    send_request_and_parse_response::<U256>(action)
}

/// Retrieves the number of transactions sent from the given address.
///
/// # Parameters
/// - `address`: The address to query the transaction count for.
/// - `tag`: Optional block ID to specify the block at which the count is queried.
///
/// # Returns
/// An `anyhow::Result<U256>` representing the number of transactions sent from the address.
pub fn get_transaction_count(address: Address, tag: Option<BlockId>) -> anyhow::Result<U256> {
    let params = serde_json::to_value((address, tag.unwrap_or_default()))?;
    let action = EthAction::Request {
        method: "eth_getTransactionCount".to_string(),
        params,
    };

    send_request_and_parse_response::<U256>(action)
}

/// Retrieves a block by its hash.
///
/// # Parameters
/// - `hash`: The hash of the block to retrieve.
/// - `full_tx`: Whether to return full transaction objects or just their hashes.
///
/// # Returns
/// An `anyhow::Result<Option<Block>>` representing the block, if found.
pub fn get_block_by_hash(hash: BlockHash, full_tx: bool) -> anyhow::Result<Option<Block>> {
    let params = serde_json::to_value((hash, full_tx))?;
    let action = EthAction::Request {
        method: "eth_getBlockByHash".to_string(),
        params,
    };

    send_request_and_parse_response::<Option<Block>>(action)
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
    number: BlockNumberOrTag,
    full_tx: bool,
) -> anyhow::Result<Option<Block>> {
    let params = serde_json::to_value((number, full_tx))?;
    let action = EthAction::Request {
        method: "eth_getBlockByNumber".to_string(),
        params,
    };

    send_request_and_parse_response::<Option<Block>>(action)
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
pub fn get_storage_at(address: Address, key: U256, tag: Option<BlockId>) -> anyhow::Result<Bytes> {
    let params = serde_json::to_value((address, key, tag.unwrap_or_default()))?;
    let action = EthAction::Request {
        method: "eth_getStorageAt".to_string(),
        params,
    };

    send_request_and_parse_response::<Bytes>(action)
}

/// Retrieves the code at a given address.
///
/// # Parameters
/// - `address`: The address of the code to query.
/// - `tag`: The block ID to specify the block at which the code is queried.
///
/// # Returns
/// An `anyhow::Result<Bytes>` representing the code stored at the given address.
pub fn get_code_at(address: Address, tag: BlockId) -> anyhow::Result<Bytes> {
    let params = serde_json::to_value((address, tag))?;
    let action = EthAction::Request {
        method: "eth_getCode".to_string(),
        params,
    };

    send_request_and_parse_response::<Bytes>(action)
}

/// Retrieves a transaction by its hash.
///
/// # Parameters
/// - `hash`: The hash of the transaction to retrieve.
///
/// # Returns
/// An `anyhow::Result<Option<Transaction>>` representing the transaction, if found.
pub fn get_transaction_by_hash(hash: TxHash) -> anyhow::Result<Option<Transaction>> {
    let params = serde_json::to_value((hash,))?;
    let action = EthAction::Request {
        method: "eth_getTransactionByHash".to_string(),
        params,
    };

    send_request_and_parse_response::<Option<Transaction>>(action)
}

/// Retrieves the receipt of a transaction by its hash.
///
/// # Parameters
/// - `hash`: The hash of the transaction for which the receipt is requested.
///
/// # Returns
/// An `anyhow::Result<Option<TransactionReceipt>>` representing the transaction receipt, if found.
pub fn get_transaction_receipt(hash: TxHash) -> anyhow::Result<Option<TransactionReceipt>> {
    let params = serde_json::to_value((hash,))?;
    let action = EthAction::Request {
        method: "eth_getTransactionReceipt".to_string(),
        params,
    };

    send_request_and_parse_response::<Option<TransactionReceipt>>(action)
}

/// Estimates the amount of gas that a transaction will consume.
///
/// # Parameters
/// - `tx`: The transaction request object containing the details of the transaction to estimate gas for.
/// - `block`: Optional block ID to specify the block at which the gas estimate should be made.
///
/// # Returns
/// An `anyhow::Result<U256>` representing the estimated gas amount.
pub fn estimate_gas(tx: TransactionRequest, block: Option<BlockId>) -> anyhow::Result<U256> {
    let params = serde_json::to_value((tx, block.unwrap_or_default()))?;
    let action = EthAction::Request {
        method: "eth_estimateGas".to_string(),
        params,
    };

    send_request_and_parse_response::<U256>(action)
}

/// Retrieves the list of accounts controlled by the node.
///
/// # Returns
/// An `anyhow::Result<Vec<Address>>` representing the list of accounts.
/// Note: This function may return an empty list depending on the node's configuration and capabilities.
pub fn get_accounts() -> anyhow::Result<Vec<Address>> {
    let action = EthAction::Request {
        method: "eth_accounts".to_string(),
        params: serde_json::Value::Array(vec![]),
    };

    send_request_and_parse_response::<Vec<Address>>(action)
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
    block_count: U256,
    last_block: BlockNumberOrTag,
    reward_percentiles: Vec<f64>,
) -> anyhow::Result<FeeHistory> {
    let params = serde_json::to_value((block_count, last_block, reward_percentiles))?;
    let action = EthAction::Request {
        method: "eth_feeHistory".to_string(),
        params,
    };

    send_request_and_parse_response::<FeeHistory>(action)
}

/// Executes a call transaction, which is directly executed in the VM of the node, but never mined into the blockchain.
///
/// # Parameters
/// - `tx`: The transaction request object containing the details of the call.
/// - `block`: Optional block ID to specify the block at which the call should be made.
///
/// # Returns
/// An `anyhow::Result<Bytes>` representing the result of the call.
pub fn call(tx: TransactionRequest, block: Option<BlockId>) -> anyhow::Result<Bytes> {
    let params = serde_json::to_value((tx, block.unwrap_or_default()))?;
    let action = EthAction::Request {
        method: "eth_call".to_string(),
        params,
    };

    send_request_and_parse_response::<Bytes>(action)
}

/// Sends a raw transaction to the network.
///
/// # Parameters
/// - `tx`: The raw transaction data.
///
/// # Returns
/// An `anyhow::Result<TxHash>` representing the hash of the transaction once it has been sent.
pub fn send_raw_transaction(tx: Bytes) -> anyhow::Result<TxHash> {
    let action = EthAction::Request {
        method: "eth_sendRawTransaction".to_string(),
        params: serde_json::to_value((tx,))?,
    };

    send_request_and_parse_response::<TxHash>(action)
}

/// Sends requests for `eth_getLogs` and `eth_subscribe` without waiting for a response, handling them as incoming `EthMessage::Sub` and `EthResponse::Response`.
///
/// # Parameters
/// - `sub_id`: The subscription ID to be used for these operations.
/// - `filter`: The filter criteria for the logs.
///
/// # Returns
/// An `anyhow::Result<()>` indicating the operation was dispatched.
pub fn getlogs_and_subscribe(sub_id: u64, filter: Filter) -> anyhow::Result<()> {
    let action = EthAction::SubscribeLogs {
        sub_id,
        kind: SubscriptionKind::Logs,
        params: Params::Logs(Box::new(filter)),
    };

    KiRequest::new()
        .target(("our", "eth", "distro", "sys"))
        .body(serde_json::to_vec(&action)?)
        .send()
}

/// Subscribes to logs without waiting for a confirmation.
///
/// # Parameters
/// - `sub_id`: The subscription ID to be used for unsubscribing.
/// - `filter`: The filter criteria for the logs.
///
/// # Returns
/// An `anyhow::Result<()>` indicating the operation was dispatched.
pub fn subscribe(sub_id: u64, filter: Filter) -> anyhow::Result<()> {
    let action = EthAction::SubscribeLogs {
        sub_id,
        kind: SubscriptionKind::Logs,
        params: Params::Logs(Box::new(filter)),
    };

    KiRequest::new()
        .target(("our", "eth", "distro", "sys"))
        .body(serde_json::to_vec(&action)?)
        .send()
}
/// Unsubscribes from a previously created subscription.
///
/// # Parameters
/// - `sub_id`: The subscription ID to unsubscribe from.
///
/// # Returns
/// An `anyhow::Result<()>` indicating the success or failure of the unsubscription request.
pub fn unsubscribe(sub_id: u64) -> anyhow::Result<()> {
    let action = EthAction::UnsubscribeLogs(sub_id);

    KiRequest::new()
        .target(("our", "eth", "distro", "sys"))
        .body(serde_json::to_vec(&action)?)
        .send()
}
