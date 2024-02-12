use crate::{Message, Request as KiRequest};
pub use alloy_primitives::{Address, BlockHash, Bytes, TxHash, U128, U256, U64, U8};
pub use alloy_rpc_types::pubsub::{Params, SubscriptionKind, SubscriptionResult};
pub use alloy_rpc_types::{
    request::{TransactionInput, TransactionRequest},
    Block, BlockId, BlockNumberOrTag, FeeHistory, Filter, Log, Transaction, TransactionReceipt,
};
use serde::{Deserialize, Serialize};

/// The Message type that can be made to eth:distro:sys. The id is used to match the response,
/// if you're not doing send_and_await.
///
/// Will be serialized and deserialized using `serde_json::to_vec` and `serde_json::from_slice`.
#[derive(Debug, Serialize, Deserialize)]
pub struct EthMessage {
    pub id: u64,
    pub action: EthAction,
}

/// The Action and Request type that can be made to eth:distro:sys.
///
/// Will be serialized and deserialized using `serde_json::to_vec` and `serde_json::from_slice`.
#[derive(Debug, Serialize, Deserialize)]
pub enum EthAction {
    /// Subscribe to logs with a custom filter. ID is to be used to unsubscribe.
    /// Logs come in as alloy_rpc_types::pubsub::SubscriptionResults
    SubscribeLogs {
        kind: SubscriptionKind,
        params: Params,
    },
    /// Kill a SubscribeLogs subscription of a given ID, to stop getting updates.
    UnsubscribeLogs,
    /// Raw request. Used by kinode_process_lib.
    Request {
        method: String,
        params: serde_json::Value,
    },
    /// Incoming subscription update.
    Sub { result: SubscriptionResult },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EthResponse {
    Ok,
    Response { value: serde_json::Value },
    Err(EthError),
}

/// The Response type which a process will get from requesting with an [`EthMessage`] will be
/// of the form `Result<(), EthError>`, serialized and deserialized using `serde_json::to_vec`
/// and `serde_json::from_slice`.
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

fn send_request_and_parse_response<T: serde::de::DeserializeOwned>(
    action: EthAction,
) -> anyhow::Result<T> {
    let msg = EthMessage {
        id: rand::random(),
        action,
    };

    let resp = KiRequest::new()
        .target(("our", "eth", "distro", "sys"))
        .body(serde_json::to_vec(&msg)?)
        .send_and_await_response(5)??;

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

pub fn get_block_number() -> anyhow::Result<u64> {
    let action = EthAction::Request {
        method: "eth_blockNumber".to_string(),
        params: ().into(),
    };

    let res = send_request_and_parse_response::<U64>(action)?;
    Ok(res.to::<u64>())
}

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

pub fn get_logs(filter: Filter) -> anyhow::Result<Vec<Log>> {
    let action = EthAction::Request {
        method: "eth_getLogs".to_string(),
        params: serde_json::to_value((filter,))?,
    };

    send_request_and_parse_response::<Vec<Log>>(action)
}

pub fn get_gas_price() -> anyhow::Result<U256> {
    let action = EthAction::Request {
        method: "eth_gasPrice".to_string(),
        params: ().into(),
    };

    send_request_and_parse_response::<U256>(action)
}

pub fn get_chain_id() -> anyhow::Result<U256> {
    let action = EthAction::Request {
        method: "eth_chainId".to_string(),
        params: ().into(),
    };

    send_request_and_parse_response::<U256>(action)
}

pub fn get_transaction_count(address: Address, tag: Option<BlockId>) -> anyhow::Result<U256> {
    let params = serde_json::to_value((address, tag.unwrap_or_default()))?;
    let action = EthAction::Request {
        method: "eth_getTransactionCount".to_string(),
        params,
    };

    send_request_and_parse_response::<U256>(action)
}

pub fn get_block_by_hash(hash: BlockHash, full_tx: bool) -> anyhow::Result<Option<Block>> {
    let params = serde_json::to_value((hash, full_tx))?;
    let action = EthAction::Request {
        method: "eth_getBlockByHash".to_string(),
        params,
    };

    send_request_and_parse_response::<Option<Block>>(action)
}

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

pub fn get_storage_at(address: Address, key: U256, tag: Option<BlockId>) -> anyhow::Result<Bytes> {
    let params = serde_json::to_value((address, key, tag.unwrap_or_default()))?;
    let action = EthAction::Request {
        method: "eth_getStorageAt".to_string(),
        params,
    };

    send_request_and_parse_response::<Bytes>(action)
}

pub fn get_code_at(address: Address, tag: BlockId) -> anyhow::Result<Bytes> {
    let params = serde_json::to_value((address, tag))?;
    let action = EthAction::Request {
        method: "eth_getCode".to_string(),
        params,
    };

    send_request_and_parse_response::<Bytes>(action)
}

pub fn get_transaction_by_hash(hash: TxHash) -> anyhow::Result<Option<Transaction>> {
    let params = serde_json::to_value((hash,))?;
    let action = EthAction::Request {
        method: "eth_getTransactionByHash".to_string(),
        params,
    };

    send_request_and_parse_response::<Option<Transaction>>(action)
}

pub fn get_transaction_receipt(hash: TxHash) -> anyhow::Result<Option<TransactionReceipt>> {
    let params = serde_json::to_value((hash,))?;
    let action = EthAction::Request {
        method: "eth_getTransactionReceipt".to_string(),
        params,
    };

    send_request_and_parse_response::<Option<TransactionReceipt>>(action)
}

pub fn estimate_gas(tx: TransactionRequest, block: Option<BlockId>) -> anyhow::Result<U256> {
    let params = serde_json::to_value((tx, block.unwrap_or_default()))?;
    let action = EthAction::Request {
        method: "eth_estimateGas".to_string(),
        params,
    };

    send_request_and_parse_response::<U256>(action)
}

// note will and should return empty I think...
pub fn get_accounts() -> anyhow::Result<Vec<Address>> {
    let action = EthAction::Request {
        method: "eth_accounts".to_string(),
        params: serde_json::Value::Array(vec![]),
    };

    send_request_and_parse_response::<Vec<Address>>(action)
}

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

pub fn call(tx: TransactionRequest, block: Option<BlockId>) -> anyhow::Result<Bytes> {
    let params = serde_json::to_value((tx, block.unwrap_or_default()))?;
    let action = EthAction::Request {
        method: "eth_call".to_string(),
        params,
    };

    send_request_and_parse_response::<Bytes>(action)
}

pub fn send_raw_transaction(tx: Bytes) -> anyhow::Result<TxHash> {
    let action = EthAction::Request {
        method: "eth_sendRawTransaction".to_string(),
        params: serde_json::to_value((tx,))?,
    };

    send_request_and_parse_response::<TxHash>(action)
}
