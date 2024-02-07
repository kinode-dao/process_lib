use crate::{Address as uqAddress, Request as uqRequest};
pub use alloy_primitives::{keccak256, Address, Bytes, FixedBytes, B256, U256, U64, U8};
pub use alloy_rpc_types::{
    AccessList, BlockNumberOrTag, CallInput, CallRequest, Filter, FilterBlockOption, FilterSet,
    Log, Topic, ValueOrArray,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct Provider<T> {
    pub handlers: HashMap<u64, Box<dyn FnMut(Vec<u8>, &mut T) + Send>>,
    pub count: u64,
}

impl<T> Provider<T> {
    pub fn new() -> Self {
        Provider {
            handlers: HashMap::new(),
            count: 0,
        }
    }

    pub fn count(&mut self) -> u64 {
        let num = self.count;
        self.count += 1;
        num
    }

    pub fn receive(&mut self, id: u64, body: Vec<u8>, state: &mut T) {
        let handler: &mut Box<dyn FnMut(Vec<u8>, &mut T) + Send> =
            self.handlers.get_mut(&id).unwrap();
        handler(body, state);
    }

    pub fn subscribe_logs(
        &mut self,
        filter: Filter,
        handler: Box<dyn FnMut(Vec<u8>, &mut T) + Send>,
    ) {
        let id = self.count();
        self.handlers.insert(id, handler);

        // generate json for getLogs and subscribeLogs, send
        self.send(
            id,
            serde_json::to_vec(&create_get_logs(filter.clone())).unwrap(),
        );
        self.send(
            id,
            serde_json::to_vec(&create_sub_logs(filter.clone())).unwrap(),
        );
    }

    pub fn call(&mut self, call: CallRequest, handler: Box<dyn FnMut(Vec<u8>, &mut T) + Send>) {
        let id = self.count();
        self.handlers.insert(id, handler);
        self.send(id, serde_json::to_vec(&create_call(call.clone())).unwrap());
    }

    pub fn gas_price(&mut self, handler: Box<dyn FnMut(Vec<u8>, &mut T) + Send>) {
        let id = self.count();
        self.handlers.insert(id, handler);
        self.send(id, serde_json::to_vec(&create_gas_price()).unwrap());
    }

    fn send(&mut self, id: u64, body: Vec<u8>) {
        let _ = uqRequest::new()
            .target(("our", "eth_provider", "eth_provider", "sys"))
            .body(body)
            .metadata(&id.to_string())
            .send();
    }
}

fn create_call(call: CallRequest) -> EthProviderRequest {
    EthProviderRequest::RpcRequest(RpcRequest {
        method: "eth_call".to_string(),
        params: serde_json::json!(vec![call]),
    })
}

fn create_sub_logs(filter: Filter) -> EthProviderRequest {
    EthProviderRequest::RpcRequest(RpcRequest {
        method: "eth_subscribe".to_string(),
        params: serde_json::json!(["logs", filter]),
    })
}

fn create_get_logs(filter: Filter) -> EthProviderRequest {
    EthProviderRequest::RpcRequest(RpcRequest {
        method: "eth_getLogs".to_string(),
        params: serde_json::json!(vec![filter]),
    })
}

fn create_gas_price() -> EthProviderRequest {
    EthProviderRequest::RpcRequest(RpcRequest {
        method: "eth_gasPrice".to_string(),
        params: serde_json::json!([]),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EthProviderRequest {
    RpcRequest(RpcRequest),
    RpcResponse(RpcResponse),
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RpcRequest {
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RpcResponse {
    pub result: serde_json::Value,
}
