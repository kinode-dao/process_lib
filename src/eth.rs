use crate::{Message, Request as KiRequest};
use alloy_primitives::{Address, BlockHash, Bytes, TxHash, U256, U64};
use alloy_rpc_types::pubsub::{Params, SubscriptionKind, SubscriptionResult};
use alloy_rpc_types::{
    Block, BlockId, BlockNumberOrTag, CallRequest, FeeHistory, Filter, Log, Transaction,
    TransactionReceipt,
};
use serde::{Deserialize, Serialize};

/// The Request type that can be made to eth:distro:sys. Currently primitive, this
/// enum will expand to support more actions in the future.
///
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
    /// Raw Json_RPC request,
    Request {
        method: String,
        params: serde_json::Value,
    },
}

/// Potential EthResponse type.
/// Can encapsulate all methods in their own response type,
/// or return generic result which can be parsed later..
#[derive(Debug, Serialize, Deserialize)]
pub enum EthResponse {
    // another possible strat, just return RpcResult<T, E, ErrResp>,
    // then try deserializing on the process_lib side.
    Ok,
    Request(serde_json::Value),
    Err(EthError),
    Sub { id: u64, result: SubscriptionResult },
}

/// The Response type which a process will get from requesting with an [`EthAction`] will be
/// of the form `Result<(), EthError>`, serialized and deserialized using `serde_json::to_vec`
/// and `serde_json::from_slice`.
#[derive(Debug, Serialize, Deserialize)]
pub enum EthError {
    /// The ethers provider threw an error when trying to subscribe
    /// (contains ProviderError serialized to debug string)
    ProviderError(String),
    SubscriptionClosed,
    /// The subscription ID was not found, so we couldn't unsubscribe.
    SubscriptionNotFound,
}

fn send_request_and_parse_response<T: serde::de::DeserializeOwned>(
    action: EthAction,
) -> anyhow::Result<T> {
    let resp = KiRequest::new()
        .target(("our", "eth", "distro", "sys"))
        .body(serde_json::to_vec(&action)?)
        .send_and_await_response(5)??;

    match resp {
        Message::Response { body, .. } => {
            let response = serde_json::from_slice::<EthResponse>(&body)?;
            match response {
                EthResponse::Request(raw) => serde_json::from_value::<T>(raw)
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

pub fn estimate_gas(tx: CallRequest, block: Option<BlockId>) -> anyhow::Result<U256> {
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

pub fn call(tx: CallRequest, block: Option<BlockId>) -> anyhow::Result<Bytes> {
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
