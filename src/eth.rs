use serde::{Deserialize, Serialize};
use ethers_core::types::{
    Address as ethAddress, 
    BlockNumber, 
    Filter, 
    FilterBlockOption, 
    H256, 
    Topic, 
    ValueOrArray,
    U64,
};
use crate::{ Address as uqAddress, Request as uqRequest };
use crate::*;

#[derive(Debug, Serialize, Deserialize)]
pub enum EthRequest {
    SubscribeLogs(SubscribeLogs)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribeLogs {
    pub filter: Filter
}

pub struct SubscribeLogsRequest {
    pub request: uqRequest,
    pub filter: Filter
}

impl SubscribeLogsRequest {

    pub fn new() -> Self {

        let request = uqRequest::new()
            .target(uqAddress::new(
                "our",
                ProcessId::new(Some("eth"), "sys", "uqbar"),
            ));

        SubscribeLogsRequest {
            request,
            filter: Filter::new(),
        }

    }

    pub fn send(mut self) -> anyhow::Result<()> {

        self.request = self.request.ipc(
            serde_json::to_vec(&EthRequest::SubscribeLogs(
                SubscribeLogs {
                    filter: self.filter.clone(),
                }
            ))?,
        );

        self.request.send()

    }

    pub fn select(mut self, filter: impl Into<FilterBlockOption>) -> Self {
        self.filter = self.filter.select(filter);
        self
    }

    pub fn address<T: Into<ValueOrArray<ethAddress>>>(mut self, address: T) -> Self {
        self.filter = self.filter.address(address);
        self
    }

    pub fn from_block<T: Into<BlockNumber>>(mut self, block: T) -> Self {
        self.filter = self.filter.from_block(block);
        self
    }

    pub fn to_block<T: Into<BlockNumber>>(mut self, block: T) -> Self {
        self.filter = self.filter.to_block(block);
        self
    }

    pub fn at_block_hash<T: Into<H256>>(mut self, hash: T) -> Self {
        self.filter = self.filter.at_block_hash(hash);
        self
    }

    pub fn event(mut self, event_name: &str) -> Self {
        self.filter = self.filter.event(event_name);
        self
    }

    pub fn events(mut self, events: impl IntoIterator<Item = impl AsRef<[u8]>>) -> Self {
        self.filter = self.filter.events(events);
        self
    }

    pub fn topic0<T: Into<Topic>>(mut self, topic: T) -> Self {
        self.filter = self.filter.topic0(topic);
        self
    }

    pub fn topic1<T: Into<Topic>>(mut self, topic: T) -> Self {
        self.filter = self.filter.topic1(topic);
        self
    }

    pub fn topic2<T: Into<Topic>>(mut self, topic: T) -> Self {
        self.filter = self.filter.topic2(topic);
        self
    }

    pub fn topic3<T: Into<Topic>>(mut self, topic: T) -> Self {
        self.filter = self.filter.topic3(topic);
        self
    }

    pub fn is_paginatable(&self) -> bool {
        self.filter.is_paginatable()
    }

    pub fn get_to_block(&self) -> Option<U64> {
        self.filter.get_to_block()
    }

}
