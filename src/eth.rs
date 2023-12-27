use crate::*;
use crate::{Address as uqAddress, Request as uqRequest};
use ethers::core::types::{Address as ethAddress, Filter, FilterBlockOption, ValueOrArray, H160};

pub struct LogSubscription {
    pub request: uqRequest,
    pub filter: Filter,
}

impl LogSubscription {
    pub fn new() -> Self {
        let request = uqRequest::new().target(Address::new(
            "our",
            ProcessId::new(Some("eth"), "sys", "uqbar"),
        ));

        LogSubscription {
            request,
            filter: Filter::new(),
        }
    }

    pub fn select(mut self, filter: impl Into<FilterBlockOption>) -> Self {
        self.filter = self.filter.select(filter);
        self
    }

    pub fn address<T: Into<ValueOrArray<ethAddress>>>(mut self, address: T) -> Self {
        self.filter = self.filter.address(address);
        self
    }
}
