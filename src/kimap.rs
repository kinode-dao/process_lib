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
pub const KIMAP_ADDRESS: &'static str = "0x29C2D0002af050d230a2567Db5f9bf43a0503773";
/// optimism chain id
pub const KIMAP_CHAIN_ID: u64 = 10;
/// first block of kimap deployment on optimism
pub const KIMAP_FIRST_BLOCK: u64 = 123_907_600;
/// the root hash of kimap, empty bytes32
pub const KIMAP_ROOT_HASH: &'static str =
    "0x0000000000000000000000000000000000000000000000000000000000000000";

/// Sol structures for Kimap requests
pub mod contract {
    use alloy_sol_macro::sol;

    sol! {
        /// Emitted when a new namespace entry is minted.
        /// - parenthash: The hash of the parent namespace entry.
        /// - childhash: The hash of the minted namespace entry's full path.
        /// - labelhash: The hash of only the label (the final entry in the path).
        /// - label: The label (the final entry in the path) of the new entry.
        event Mint(
            bytes32 indexed parenthash,
            bytes32 indexed childhash,
            bytes indexed labelhash,
            bytes label
        );

        /// Emitted when a fact is created on an existing namespace entry.
        /// Facts are immutable and may only be written once. A fact label is
        /// prepended with an exclamation mark (!) to indicate that it is a fact.
        /// - parenthash The hash of the parent namespace entry.
        /// - facthash The hash of the newly created fact's full path.
        /// - labelhash The hash of only the label (the final entry in the path).
        /// - label The label of the fact.
        /// - data The data stored at the fact.
        event Fact(
            bytes32 indexed parenthash,
            bytes32 indexed facthash,
            bytes indexed labelhash,
            bytes label,
            bytes data
        );

        /// Emitted when a new note is created on an existing namespace entry.
        /// Notes are mutable. A note label is prepended with a tilde (~) to indicate
        /// that it is a note.
        /// - parenthash: The hash of the parent namespace entry.
        /// - notehash: The hash of the newly created note's full path.
        /// - labelhash: The hash of only the label (the final entry in the path).
        /// - label: The label of the note.
        /// - data: The data stored at the note.
        event Note(
            bytes32 indexed parenthash,
            bytes32 indexed notehash,
            bytes indexed labelhash,
            bytes label,
            bytes data
        );

        /// Emitted when a gene is set for an existing namespace entry.
        /// A gene is a specific TBA implementation which will be applied to all
        /// sub-entries of the namespace entry.
        /// - entry: The namespace entry's namehash.
        /// - gene: The address of the TBA implementation.
        event Gene(bytes32 indexed entry, address indexed gene);

        /// Emitted when the zeroth namespace entry is minted.
        /// Occurs exactly once at initialization.
        /// - zeroTba: The address of the zeroth TBA
        event Zero(address indexed zeroTba);

        /// Emitted when a namespace entry is transferred from one address
        /// to another.
        /// - from: The address of the sender.
        /// - to: The address of the recipient.
        /// - id: The namehash of the namespace entry (converted to uint256).
        event Transfer(
            address indexed from,
            address indexed to,
            uint256 indexed id
        );

        /// Emitted when a namespace entry is approved for transfer.
        /// - owner: The address of the owner.
        /// - spender: The address of the spender.
        /// - id: The namehash of the namespace entry (converted to uint256).
        event Approval(
            address indexed owner,
            address indexed spender,
            uint256 indexed id
        );

        /// Emitted when an operator is approved for all of an owner's
        /// namespace entries.
        /// - owner: The address of the owner.
        /// - operator: The address of the operator.
        /// - approved: Whether the operator is approved.
        event ApprovalForAll(
            address indexed owner,
            address indexed operator,
            bool approved
        );

        /// Retrieves information about a specific namespace entry.
        /// - namehash The namehash of the namespace entry to query.
        ///
        /// Returns:
        /// - tba: The address of the token-bound account associated
        /// with the entry.
        /// - owner: The address of the entry owner.
        /// - data: The note or fact bytes associated with the entry
        /// (empty if not a note or fact).
        function get(
            bytes32 namehash
        ) external view returns (address tba, address owner, bytes memory data);

        /// Mints a new namespace entry and creates a token-bound account for
        /// it. Must be called by a parent namespace entry token-bound account.
        /// - who: The address to own the new namespace entry.
        /// - label: The label to mint beneath the calling parent entry.
        /// - initialization: Initialization calldata applied to the new
        /// minted entry's token-bound account.
        /// - erc721Data: ERC-721 data -- passed to comply with
        /// `ERC721TokenReceiver.onERC721Received()`.
        /// - implementation: The address of the implementation contract for
        /// the token-bound account: this will be overriden by the gene if the
        /// parent entry has one set.
        ///
        /// Returns:
        /// - tba: The address of the new entry's token-bound account.
        function mint(
            address who,
            bytes calldata label,
            bytes calldata initialization,
            bytes calldata erc721Data,
            address implementation
        ) external returns (address tba);

        /// Sets the gene for the calling namespace entry.
        /// - _gene: The address of the TBA implementation to set for all
        /// children of the calling namespace entry.
        function gene(address _gene) external;

        /// Creates a new fact beneath the calling namespace entry.
        /// - fact: The fact label to create. Must be prepended with an
        /// exclamation mark (!).
        /// - data: The data to be stored at the fact.
        ///
        /// Returns:
        /// - facthash: The namehash of the newly created fact.
        function fact(
            bytes calldata fact,
            bytes calldata data
        ) external returns (bytes32 facthash);

        /// Creates a new note beneath the calling namespace entry.
        /// - note: The note label to create. Must be prepended with a tilde (~).
        /// - data: The data to be stored at the note.
        ///
        /// Returns:
        /// - notehash: The namehash of the newly created note.
        function note(
            bytes calldata note,
            bytes calldata data
        ) external returns (bytes32 notehash);

        /// Retrieves the token-bound account address of a namespace entry.
        /// - entry: The entry namehash (as uint256) for which to get the
        /// token-bound account.
        ///
        /// Returns:
        /// - tba: The token-bound account address of the namespace entry.
        function tbaOf(uint256 entry) external view returns (address tba);

        function balanceOf(address owner) external view returns (uint256);

        function getApproved(uint256 entry) external view returns (address);

        function isApprovedForAll(
            address owner,
            address operator
        ) external view returns (bool);

        function ownerOf(uint256 entry) external view returns (address);

        function setApprovalForAll(address operator, bool approved) external;

        function approve(address spender, uint256 entry) external;

        function safeTransferFrom(address from, address to, uint256 id) external;

        function safeTransferFrom(
            address from,
            address to,
            uint256 id,
            bytes calldata data
        ) external;

        function transferFrom(address from, address to, uint256 id) external;

        function supportsInterface(bytes4 interfaceId) external view returns (bool);

        /// Retrieves the address of the ERC-6551 implementation of the
        /// zeroth entry. This is set once at initialization.
        ///
        /// Returns:
        /// - implementation: The address of the ERC-6551 implementation.
        function get6551Implementation() external view returns (address);
    }
}

/// A mint log from the kimap, converted to a 'resolved' format using
/// namespace data saved in the kns_indexer.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Mint {
    pub name: String,
    pub parent_path: String,
}

/// A note log from the kimap, converted to a 'resolved' format using
/// namespace data saved in the kns_indexer
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Note {
    pub note: String,
    pub parent_path: String,
    pub data: Bytes,
}

/// Errors that can occur when decoding a log from the kimap using
/// [`decode_mint_log`] or [`decode_note_log`].
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum DecodeLogError {
    /// The log's topic is not a mint or note event.
    UnexpectedTopic(B256),
    /// The name is not valid (according to [`valid_name`]).
    InvalidName(String),
    /// An error occurred while decoding the log.
    DecodeError(String),
    /// The parent name could not be resolved with `kns_indexer`.
    UnresolvedParent(String),
}

/// Canonical function to determine if a kimap entry is valid. This should
/// be used whenever reading a new kimap entry from a mints query, because
/// while most frontends will enforce these rules, it is possible to post
/// invalid names to the kimap contract.
///
/// This checks a **single name**, not the full path-name. A full path-name
/// is comprised of valid names separated by `.`
pub fn valid_name(name: &str, note: bool) -> bool {
    if note {
        name.is_ascii()
            && name.len() >= 2
            && name.chars().next() == Some('~')
            && name
                .chars()
                .skip(1)
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    } else {
        name.is_ascii()
            && name.len() >= 1
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }
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
///
/// Uses `valid_name` to check if the name is valid.
pub fn decode_mint_log(log: &crate::eth::Log) -> Result<Mint, DecodeLogError> {
    let contract::Note::SIGNATURE_HASH = log.topics()[0] else {
        return Err(DecodeLogError::UnexpectedTopic(log.topics()[0]));
    };
    let decoded = contract::Mint::decode_log_data(log.data(), true)
        .map_err(|e| DecodeLogError::DecodeError(e.to_string()))?;
    let name = String::from_utf8_lossy(&decoded.label).to_string();
    if !valid_name(&name, false) {
        return Err(DecodeLogError::InvalidName(name));
    }
    match resolve_parent(log, None) {
        Some(parent_path) => Ok(Mint { name, parent_path }),
        None => Err(DecodeLogError::UnresolvedParent(name)),
    }
}

/// Decode a note log from the kimap into a 'resolved' format.
///
/// Uses `valid_name` to check if the name is valid.
pub fn decode_note_log(log: &crate::eth::Log) -> Result<Note, DecodeLogError> {
    let contract::Note::SIGNATURE_HASH = log.topics()[0] else {
        return Err(DecodeLogError::UnexpectedTopic(log.topics()[0]));
    };
    let decoded = contract::Note::decode_log_data(log.data(), true)
        .map_err(|e| DecodeLogError::DecodeError(e.to_string()))?;
    let note = String::from_utf8_lossy(&decoded.label).to_string();
    if !valid_name(&note, true) {
        return Err(DecodeLogError::InvalidName(note));
    }
    match resolve_parent(log, None) {
        Some(parent_path) => Ok(Note {
            note,
            parent_path,
            data: decoded.data,
        }),
        None => Err(DecodeLogError::UnresolvedParent(note)),
    }
}

/// Given a [`crate::eth::Log`] (which must be a log from kimap), resolve the parent name
/// of the new entry or note.
pub fn resolve_parent(log: &crate::eth::Log, timeout: Option<u64>) -> Option<String> {
    let parent_hash = log.topics()[1].to_string();
    net::get_name(&parent_hash, log.block_number, timeout)
}

/// Given a [`crate::eth::Log`] (which must be a log from kimap), resolve the full name
/// of the new entry or note.
///
/// Uses `valid_name` to check if the name is valid.
pub fn resolve_full_name(log: &crate::eth::Log, timeout: Option<u64>) -> Option<String> {
    let parent_hash = log.topics()[1].to_string();
    let parent_name = net::get_name(&parent_hash, log.block_number, timeout)?;
    let log_name = match log.topics()[0] {
        contract::Mint::SIGNATURE_HASH => {
            let decoded = contract::Mint::decode_log_data(log.data(), true).unwrap();
            decoded.label
        }
        contract::Note::SIGNATURE_HASH => {
            let decoded = contract::Note::decode_log_data(log.data(), true).unwrap();
            decoded.label
        }
        _ => return None,
    };
    let name = String::from_utf8_lossy(&log_name);
    if !valid_name(&name, log.topics()[0] == contract::Note::SIGNATURE_HASH) {
        return None;
    }
    Some(format!("{name}.{parent_name}"))
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
            namehash: FixedBytes::<32>::from_str(&namehash(path))
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

        Ok((res.tba, res.owner, note_data))
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
            namehash: FixedBytes::<32>::from_str(entryhash).map_err(|_| EthError::InvalidParams)?,
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

        Ok((res.tba, res.owner, note_data))
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
