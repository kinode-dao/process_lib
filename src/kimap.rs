use crate::eth::{EthError, Provider};
use crate::kimap::contract::getCall;
use crate::net;
use alloy::rpc::types::request::{TransactionInput, TransactionRequest};
use alloy::{hex, primitives::keccak256};
use alloy_primitives::{Address, Bytes, FixedBytes, B256};
use alloy_sol_types::{SolCall, SolEvent, SolValue};
use serde::{Deserialize, Serialize};
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

/// Sol structures for Kimap requests
pub mod contract {
    use alloy_sol_macro::sol;

    sol! {
        event Mint(bytes32 indexed parenthash, bytes32 indexed childhash, bytes indexed labelhash, bytes name);
        event Note(bytes32 indexed parenthash, bytes32 indexed notehash, bytes indexed labelhash, bytes note, bytes data);

        function get (
            bytes32 entryhash
        ) external view returns (
            address tokenBoundAccount,
            address tokenOwner,
            bytes memory data
        );
    }
}

/// A mint log from the kimap, converted to a 'resolved' format using
/// namespace data saved in the kns_indexer.
pub struct Mint {
    pub name: String,
    pub parent_path: String,
}

/// A note log from the kimap, converted to a 'resolved' format using
/// namespace data saved in the kns_indexer
pub struct Note {
    pub note: String,
    pub parent_path: String,
    pub data: Bytes,
}

/// Produce a namehash from a kimap name.
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

/// Decode a mint log from the kimap into a 'resolved' format.
pub fn decode_mint_log(log: &crate::eth::Log) -> Option<Mint> {
    let contract::Note::SIGNATURE_HASH = log.topics()[0] else {
        return None;
    };
    let decoded = contract::Mint::decode_log_data(log.data(), true).ok()?;
    Some(Mint {
        name: decoded.name.to_string(),
        parent_path: resolve_parent(log, None)?,
    })
}

/// Decode a note log from the kimap into a 'resolved' format.
pub fn decode_note_log(log: &crate::eth::Log) -> Option<Note> {
    let contract::Note::SIGNATURE_HASH = log.topics()[0] else {
        return None;
    };
    let decoded = contract::Note::decode_log_data(log.data(), true).ok()?;
    Some(Note {
        note: decoded.note.to_string(),
        parent_path: resolve_parent(log, None)?,
        data: decoded.data,
    })
}

/// Given a [`crate::eth::Log`] (which must be a log from kimap), resolve the parent name
/// of the new entry or note.
pub fn resolve_parent(log: &crate::eth::Log, timeout: Option<u64>) -> Option<String> {
    let parent_hash = log.topics()[1].to_string();
    net::get_name(&parent_hash, log.block_number, timeout)
}

/// Given a [`crate::eth::Log`] (which must be a log from kimap), resolve the full name
/// of the new entry or note.
pub fn resolve_full_name(log: &crate::eth::Log, timeout: Option<u64>) -> Option<String> {
    let parent_hash = log.topics()[1].to_string();
    let parent_name = net::get_name(&parent_hash, log.block_number, timeout)?;
    let log_name = match log.topics()[0] {
        contract::Mint::SIGNATURE_HASH => {
            let decoded = contract::Mint::decode_log_data(log.data(), true).unwrap();
            decoded.name
        }
        contract::Note::SIGNATURE_HASH => {
            let decoded = contract::Note::decode_log_data(log.data(), true).unwrap();
            decoded.note
        }
        _ => return None,
    };
    Some(format!(
        "{}.{}",
        String::from_utf8_lossy(&log_name),
        parent_name
    ))
}

/// Helper struct for reading from the kimap.
#[derive(Clone, Debug, Deserialize, Serialize)]
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

    /// Create a filter for all mint events.
    pub fn mint_filter(&self) -> crate::eth::Filter {
        crate::eth::Filter::new()
            .address(self.address)
            .event(contract::Mint::SIGNATURE)
    }

    /// Create a filter for all note events.
    pub fn note_filter(&self) -> crate::eth::Filter {
        crate::eth::Filter::new()
            .address(self.address)
            .event(contract::Note::SIGNATURE)
    }

    /// Create a filter for a given set of specific notes. This function will
    /// hash the note labels and use them as the topic3 filter.
    ///
    /// Example:
    /// ```rust
    /// let filter = kimap.notes_filter(&["~note1", "~note2"]);
    /// ```
    pub fn notes_filter(&self, notes: &[&str]) -> crate::eth::Filter {
        self.note_filter().topic3(
            notes
                .into_iter()
                .map(|note| keccak256(note))
                .collect::<Vec<_>>(),
        )
    }
}
