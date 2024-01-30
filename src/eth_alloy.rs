use crate::*;
use crate::{Address as uqAddress, Request as uqRequest};
pub use alloy_primitives::{keccak256, Address, B256, U256, U64, U8};
pub use alloy_rpc_types::{
    AccessList, BlockNumberOrTag, CallInput, CallRequest, Filter, FilterBlockOption, FilterSet,
    Log as AlloyLog, Topic, ValueOrArray,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct Provider {
    pub closures: HashMap<u64, Box<dyn FnMut(Vec<u8>) + Send>>,
    pub count: u64,
}

impl Provider {
    pub fn new() -> Self {
        Provider {
            closures: HashMap::new(),
            count: 0,
        }
    }

    pub fn count(&mut self) -> u64 {
        let num = self.count;
        self.count += 1;
        num
    }

    pub fn subscribe_logs(
        &mut self,
        method: ProviderMethod,
        closure: Box<dyn FnMut(Vec<u8>) + Send>,
    ) {
        let id = self.count();
        self.closures.insert(id, closure);
        self.send(id, method)
    }

    pub fn call(&mut self, method: ProviderMethod, closure: Box<dyn FnMut(Vec<u8>) + Send>) {
        let id = self.count();
        self.closures.insert(id, closure);
        self.send(id, method)
    }

    pub fn receive(&mut self, id: u64, body: Vec<u8>) {
        let closure: &mut Box<dyn FnMut(Vec<u8>) + Send> = self.closures.get_mut(&id).unwrap();
        closure(body);
    }

    fn send(&mut self, id: u64, method: ProviderMethod) {
        let _ = uqRequest::new()
            .target(("our", "eth_provider", "eth_provider", "sys"))
            .body(method.get_provider_request_body())
            .metadata(&id.to_string())
            .send();
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ProviderMethod {
    SubscribeLogs(AlloySubscribeLogsRequest),
    Call(AlloyCallRequest),
}

trait ProviderMethodTrait {
    fn get_provider_request_body(&self) -> Vec<u8>;
}

impl ProviderMethodTrait for ProviderMethod {
    fn get_provider_request_body(&self) -> Vec<u8> {
        match self {
            ProviderMethod::SubscribeLogs(method) => method.get_provider_request_body(),
            ProviderMethod::Call(method) => method.get_provider_request_body(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EthProviderRequests {
    RpcRequest(RpcRequest),
    RpcResponse(RpcResponse),
    Test,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RpcRequest {
    pub method: String,
    pub params: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RpcResponse {
    pub result: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AlloyCallRequest {
    pub call_request: CallRequest,
}

impl AlloyCallRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_provider_request_body(&self) -> Vec<u8> {
        serde_json::to_vec(&EthProviderRequests::RpcRequest(RpcRequest {
            method: "eth_call".to_string(),
            params: serde_json::json!(self.call_request).to_string(),
        }))
        .expect("Could not serialize request body")
    }

    pub fn from(mut self, addr: Address) -> Self {
        self.call_request.from = Some(addr);
        self
    }

    pub fn to(mut self, addr: Address) -> Self {
        self.call_request.to = Some(addr);
        self
    }

    pub fn gas_price(mut self, gas_price: U256) -> Self {
        self.call_request.gas_price = Some(gas_price);
        self
    }

    pub fn max_fee_per_gas(mut self, max_fee_per_gas: U256) -> Self {
        self.call_request.max_fee_per_gas = Some(max_fee_per_gas);
        self
    }

    pub fn max_priority_fee_per_gas(mut self, max_priority_fee_per_gas: U256) -> Self {
        self.call_request.max_priority_fee_per_gas = Some(max_priority_fee_per_gas);
        self
    }

    pub fn gas(mut self, gas: U256) -> Self {
        self.call_request.gas = Some(gas);
        self
    }

    pub fn value(mut self, value: U256) -> Self {
        self.call_request.value = Some(value);
        self
    }

    pub fn input(mut self, input: CallInput) -> Self {
        self.call_request.input = input;
        self
    }

    pub fn nonce(mut self, nonce: U64) -> Self {
        self.call_request.nonce = Some(nonce);
        self
    }

    pub fn chain_id(mut self, chain_id: U64) -> Self {
        self.call_request.chain_id = Some(chain_id);
        self
    }

    pub fn access_list(mut self, access_list: AccessList) -> Self {
        self.call_request.access_list = Some(access_list);
        self
    }

    pub fn max_fee_per_blob_gas(mut self, max_fee_per_blob_gas: U256) -> Self {
        self.call_request.max_fee_per_blob_gas = Some(max_fee_per_blob_gas);
        self
    }

    pub fn blob_versioned_hashes(mut self, blob_versioned_hashes: Vec<B256>) -> Self {
        self.call_request.blob_versioned_hashes = Some(blob_versioned_hashes);
        self
    }

    pub fn transaction_type(mut self, transaction_type: U8) -> Self {
        self.call_request.transaction_type = Some(transaction_type);
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AlloySubscribeLogsRequest {
    pub filter: Filter,
}

impl AlloySubscribeLogsRequest {
    pub fn send(self) -> anyhow::Result<()> {
        uqRequest::new()
            .target(("our", "eth_provider", "eth_provider", "sys"))
            .body(self.get_provider_request_body())
            .send()
    }

    pub fn get_provider_request_body(&self) -> Vec<u8> {
        serde_json::to_vec(&EthProviderRequests::RpcRequest(RpcRequest {
            method: "eth_subscribe".to_string(),
            params: serde_json::json!(["logs", self.filter]).to_string(),
        }))
        .expect("Could not serialize request body")
    }

    /// Creates a new, empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the inner filter object
    ///
    /// *NOTE:* ranges are always inclusive
    ///
    /// # Examples
    ///
    /// Match only a specific block
    ///
    /// ```rust
    /// # use alloy_rpc_types::Filter;
    /// # fn main() {
    ///
    /// let filter = Filter::new().select(69u64);
    /// # }
    /// ```
    /// This is the same as `Filter::new().from_block(1337u64).to_block(1337u64)`
    ///
    /// Match the latest block only
    ///
    /// ```rust
    /// # use alloy_rpc_types::BlockNumberOrTag;
    /// # use alloy_rpc_types::Filter;
    /// # fn main() {
    /// let filter = Filter::new().select(BlockNumberOrTag::Latest);
    /// # }
    /// ```
    ///
    /// Match a block by its hash
    ///
    /// ```rust
    /// # use alloy_primitives::B256;
    /// # use alloy_rpc_types::Filter;
    /// # fn main() {
    /// let filter = Filter::new().select(B256::ZERO);
    /// # }
    /// ```
    /// This is the same as `at_block_hash`
    ///
    /// Match a range of blocks
    ///
    /// ```rust
    /// # use alloy_rpc_types::Filter;
    /// # fn main() {
    /// let filter = Filter::new().select(0u64..100u64);
    /// # }
    /// ```
    ///
    /// Match all blocks in range `(1337..BlockNumberOrTag::Latest)`
    ///
    /// ```rust
    /// # use alloy_rpc_types::Filter;
    /// # fn main() {
    /// let filter = Filter::new().select(1337u64..);
    /// # }
    /// ```
    ///
    /// Match all blocks in range `(BlockNumberOrTag::Earliest..1337)`
    ///
    /// ```rust
    /// # use alloy_rpc_types::Filter;
    /// # fn main() {
    /// let filter = Filter::new().select(..1337u64);
    /// # }
    /// ```
    #[must_use]
    pub fn select(mut self, filter: impl Into<FilterBlockOption>) -> Self {
        self.filter.block_option = filter.into();
        self
    }

    /// Sets the from block number
    #[allow(clippy::wrong_self_convention)]
    #[must_use]
    pub fn from_block<T: Into<BlockNumberOrTag>>(mut self, block: T) -> Self {
        self.filter.block_option = self.filter.block_option.set_from_block(block.into());
        self
    }

    /// Sets the to block number
    #[allow(clippy::wrong_self_convention)]
    #[must_use]
    pub fn to_block<T: Into<BlockNumberOrTag>>(mut self, block: T) -> Self {
        self.filter.block_option = self.filter.block_option.set_to_block(block.into());
        self
    }

    /// Pins the block hash for the filter
    #[must_use]
    pub fn at_block_hash<T: Into<B256>>(mut self, hash: T) -> Self {
        self.filter.block_option = self.filter.block_option.set_hash(hash.into());
        self
    }
    /// Sets the inner filter object
    ///
    /// *NOTE:* ranges are always inclusive
    ///
    /// # Examples
    ///
    /// Match only a specific address `("0xAc4b3DacB91461209Ae9d41EC517c2B9Cb1B7DAF")`
    ///
    /// ```rust
    /// # use alloy_primitives::Address;
    /// # use alloy_rpc_types::Filter;
    /// # fn main() {
    /// let filter = Filter::new()
    ///     .address("0xAc4b3DacB91461209Ae9d41EC517c2B9Cb1B7DAF".parse::<Address>().unwrap());
    /// # }
    /// ```
    ///
    /// Match all addresses in array `(vec!["0xAc4b3DacB91461209Ae9d41EC517c2B9Cb1B7DAF",
    /// "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8"])`
    ///
    /// ```rust
    /// # use alloy_primitives::Address;
    /// # use alloy_rpc_types::Filter;
    /// # fn main() {
    /// let addresses = vec![
    ///     "0xAc4b3DacB91461209Ae9d41EC517c2B9Cb1B7DAF".parse::<Address>().unwrap(),
    ///     "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8".parse::<Address>().unwrap(),
    /// ];
    /// let filter = Filter::new().address(addresses);
    /// # }
    /// ```
    #[must_use]
    pub fn address<T: Into<ValueOrArray<Address>>>(mut self, address: T) -> Self {
        self.filter.address = address.into().into();
        self
    }

    /// Given the event signature in string form, it hashes it and adds it to the topics to monitor
    #[must_use]
    pub fn event(self, event_name: &str) -> Self {
        let hash = keccak256(event_name.as_bytes());
        self.event_signature(hash)
    }

    /// Hashes all event signatures and sets them as array to event_signature(topic0)
    #[must_use]
    pub fn events(self, events: impl IntoIterator<Item = impl AsRef<[u8]>>) -> Self {
        let events = events
            .into_iter()
            .map(|e| keccak256(e.as_ref()))
            .collect::<Vec<_>>();
        self.event_signature(events)
    }

    /// Sets event_signature(topic0) (the event name for non-anonymous events)
    #[must_use]
    pub fn event_signature<T: Into<Topic>>(mut self, topic: T) -> Self {
        self.filter.topics[0] = topic.into();
        self
    }

    /// Sets topic0 (the event name for non-anonymous events)
    #[must_use]
    #[deprecated(note = "use `event_signature` instead")]
    pub fn topic0<T: Into<Topic>>(mut self, topic: T) -> Self {
        self.filter.topics[0] = topic.into();
        self
    }

    /// Sets the 1st indexed topic
    #[must_use]
    pub fn topic1<T: Into<Topic>>(mut self, topic: T) -> Self {
        self.filter.topics[1] = topic.into();
        self
    }

    /// Sets the 2nd indexed topic
    #[must_use]
    pub fn topic2<T: Into<Topic>>(mut self, topic: T) -> Self {
        self.filter.topics[2] = topic.into();
        self
    }

    /// Sets the 3rd indexed topic
    #[must_use]
    pub fn topic3<T: Into<Topic>>(mut self, topic: T) -> Self {
        self.filter.topics[3] = topic.into();
        self
    }

    /// Returns true if this is a range filter and has a from block
    pub fn is_paginatable(&self) -> bool {
        self.filter.get_from_block().is_some()
    }

    /// Returns the numeric value of the `toBlock` field
    pub fn get_to_block(&self) -> Option<u64> {
        self.filter
            .block_option
            .get_to_block()
            .and_then(|b| b.as_number())
    }

    /// Returns the numeric value of the `fromBlock` field
    pub fn get_from_block(&self) -> Option<u64> {
        self.filter
            .block_option
            .get_from_block()
            .and_then(|b| b.as_number())
    }

    /// Returns the numeric value of the `fromBlock` field
    pub fn get_block_hash(&self) -> Option<B256> {
        match self.filter.block_option {
            FilterBlockOption::AtBlockHash(hash) => Some(hash),
            FilterBlockOption::Range { .. } => None,
        }
    }

    /// Returns true if at least one topic is set
    pub fn has_topics(&self) -> bool {
        self.filter.topics.iter().any(|t| !t.is_empty())
    }
}
