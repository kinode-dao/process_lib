use crate::eth::*;
use crate::kimap::contract::getCall;
use alloy::rpc::types::request::{TransactionInput, TransactionRequest};
use alloy::{hex, primitives::keccak256};
use alloy_primitives::FixedBytes;
use alloy_primitives::B256;
use alloy_primitives::{Address, Bytes};
use alloy_sol_types::SolCall;
use alloy_sol_types::SolValue;
use std::str::FromStr;

/// kimap deployment address on optimism
pub const KIMAP_ADDRESS: &'static str = "0x7290Aa297818d0b9660B2871Bb87f85a3f9B4559";
/// optimism chain id
pub const KIMAP_CHAIN_ID: u64 = 10;
/// first block of kimap deployment on optimism
pub const KIMAP_FIRST_BLOCK: u64 = 114_923_786;
/// the root hash of kimap, empty bytes32
pub const KIMAP_ROOT_HASH: &'static str =
    "0x0000000000000000000000000000000000000000000000000000000000000000";

// Sol structures for Kimap requests
pub mod contract {
    use alloy_sol_macro::sol;

    sol! {
        event Mint(bytes32 indexed parenthash, bytes32 indexed childhash, bytes indexed labelhash, bytes name);
        event Note(bytes32 indexed nodehash, bytes32 indexed notehash, bytes indexed labelhash, bytes note, bytes data);

        function get (
            bytes32 entryhash
        ) external view returns (
            address tokenBoundAccount,
            address tokenOwner,
            bytes memory data
        );
    }
}

pub fn namehash(name: &str) -> String {
    let mut node = B256::default();

    let mut labels: Vec<&str> = name.split('.').collect();
    labels.reverse();

    for label in labels.iter() {
        let l = keccak256(label);
        node = keccak256((node, l).abi_encode_packed());
    }
    format!("0x{}", hex::encode(node))
}

/// Helper struct for the Kimap.
pub struct Kimap {
    pub provider: Provider,
    address: Address,
}

impl Kimap {
    /// Creates a new Kimap instance with a specified address.
    ///
    /// # Arguments
    /// * `provider` - A reference to the Provider.
    /// * `address` - The address of the Kimap contract.
    pub fn new(provider: Provider, address: Address) -> Self {
        Self { provider, address }
    }

    /// Creates a new Kimap instance with the default address and chain ID.
    pub fn default(timeout: u64) -> Self {
        let provider = Provider::new(KIMAP_CHAIN_ID, timeout);
        Self::new(provider, Address::from_str(KIMAP_ADDRESS).unwrap())
    }

    /// Returns the in-use Kimap contract address.
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Gets an entry from the Kimap by its string-formatted name.
    ///
    /// # Parameters
    /// - `path`: The name-path to get from the Kimap.
    /// # Returns
    /// A `Result<(Address, Address, Option<Bytes>), EthError>` representing the TBA, owner,
    /// and value if the entry exists and is a note.
    pub fn get(&self, path: &str) -> Result<(Address, Address, Option<Bytes>), EthError> {
        let get_call = getCall {
            entryhash: FixedBytes::<32>::from_str(&namehash(path))
                .map_err(|_| EthError::InvalidParams)?,
        }
        .abi_encode();

        let tx_req = TransactionRequest::default()
            .input(TransactionInput::new(get_call.into()))
            .to(self.address);

        let res_bytes = self.provider.call(tx_req, None)?;

        let res = getCall::abi_decode_returns(&res_bytes, false)
            .map_err(|_| EthError::RpcMalformedResponse)?;

        let note_data = if res.data == Bytes::default() {
            None
        } else {
            Some(res.data)
        };

        Ok((res.tokenBoundAccount, res.tokenOwner, note_data))
    }

    /// Gets an entry from the Kimap by its hash.
    ///
    /// # Parameters
    /// - `entryhash`: The entry to get from the Kimap.
    /// # Returns
    /// A `Result<(Address, Address, Option<Bytes>), EthError>` representing the TBA, owner,
    /// and value if the entry exists and is a note.
    pub fn get_hash(&self, entryhash: &str) -> Result<(Address, Address, Option<Bytes>), EthError> {
        let get_call = getCall {
            entryhash: FixedBytes::<32>::from_str(entryhash)
                .map_err(|_| EthError::InvalidParams)?,
        }
        .abi_encode();

        let tx_req = TransactionRequest::default()
            .input(TransactionInput::new(get_call.into()))
            .to(self.address);

        let res_bytes = self.provider.call(tx_req, None)?;

        let res = getCall::abi_decode_returns(&res_bytes, false)
            .map_err(|_| EthError::RpcMalformedResponse)?;

        let note_data = if res.data == Bytes::default() {
            None
        } else {
            Some(res.data)
        };

        Ok((res.tokenBoundAccount, res.tokenOwner, note_data))
    }
}
