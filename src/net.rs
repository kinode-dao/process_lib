use crate::{get_blob, Address, NodeId, Request, SendError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

//
// Networking protocol types and functions for interacting with it
//

/// The data structure used by `net:distro:sys` and the rest of the runtime to
/// represent node identities in the KNS (Kinode Name System).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub name: NodeId,
    pub networking_key: String,
    pub routing: NodeRouting,
}

/// Routing information for a node identity. Produced from kimap data entries
/// and used to create net connections between nodes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NodeRouting {
    /// Indirect nodes have routers which resolve connections for them.
    Routers(Vec<NodeId>),
    /// Direct nodes publish an IP address and a set of ports corresponding
    /// to protocols they support for incoming connections.
    Direct {
        ip: String,
        ports: BTreeMap<String, u16>,
    },
}

impl Identity {
    /// Check if an identity is a direct node.
    pub fn is_direct(&self) -> bool {
        matches!(&self.routing, NodeRouting::Direct { .. })
    }
    /// Get the port used by a direct node for a given protocol,
    /// if the node is direct and supports the protocol.
    ///
    /// Protocols are represented by a string code such as "ws", "tcp", "udp".
    pub fn get_protocol_port(&self, protocol: &str) -> Option<u16> {
        match &self.routing {
            NodeRouting::Routers(_) => None,
            NodeRouting::Direct { ports, .. } => ports.get(protocol).cloned(),
        }
    }
    /// Get the list of routers for an indirect node.
    pub fn routers(&self) -> Option<&Vec<NodeId>> {
        match &self.routing {
            NodeRouting::Routers(routers) => Some(routers),
            NodeRouting::Direct { .. } => None,
        }
    }
}

/// Must be parsed from message pack vector (use `rmp-serde`).
/// All "Get" actions must be sent from a local process. Used for debugging.
///
/// Sending a NetAction requires messaging capabilities to `net:distro:sys`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetAction {
    /// Received from a router of ours when they have a new pending passthrough
    /// for us. We should respond (if we desire) by using them to initialize a
    /// routed connection with the NodeId given.
    ///
    /// This cannot be sent locally.
    ConnectionRequest(NodeId),
    /// Can only receive from trusted source: requires net root [`crate::Capability`].
    KnsUpdate(KnsUpdate),
    /// Can only receive from trusted source: requires net root [`crate::Capability`].
    KnsBatchUpdate(Vec<KnsUpdate>),
    /// Get a list of peers with whom we have an open connection.
    GetPeers,
    /// Get the [`Identity`] struct for a single peer.
    GetPeer(String),
    /// Get a user-readable diagnostics string containing networking information.
    GetDiagnostics,
    /// Sign the attached blob payload with our node's networking key.
    /// **Only accepted from our own node.**
    /// **The source [`Address`] will always be prepended to the payload.**
    Sign,
    /// Given a message in blob payload, verify the message is signed by
    /// the given source. If the signer is not in our representation of
    /// the PKI, will not verify.
    /// **The `from` [`Address`] will always be prepended to the payload.**
    Verify { from: Address, signature: Vec<u8> },
}

/// Must be parsed from message pack vector (use `rmp-serde`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetResponse {
    /// Response to [`NetAction::ConnectionRequest`].
    Accepted(NodeId),
    /// Response to [`NetAction::ConnectionRequest`].
    Rejected(NodeId),
    /// Response to [`NetAction::GetPeers`]
    Peers(Vec<Identity>),
    /// Response to [`NetAction::GetPeer`]
    Peer(Option<Identity>),
    /// Response to [`NetAction::GetDiagnostics`]. a user-readable string.
    Diagnostics(String),
    /// Response to [`NetAction::Sign`]. Contains the signature in blob.
    Signed,
    /// Response to [`NetAction::Verify`]. Boolean indicates whether
    /// the signature was valid or not. Note that if the signer node
    /// cannot be found in our representation of PKI, this will return false,
    /// because we cannot find the networking public key to verify with.
    Verified(bool),
}

/// Request performed to `kns-indexer:kns-indexer:sys`, a userspace process
/// installed by default.
///
/// Other requests exist but are only used internally.
#[derive(Debug, Serialize, Deserialize)]
pub enum IndexerRequests {
    /// Get the name associated with a namehash. This is used to resolve namehashes
    /// from events in the `kimap` contract.
    NamehashToName(NamehashToNameRequest),
}

/// Request to resolve a namehash to a name. Hash is a namehash from `kimap`.
/// Block is optional, and if provided will return the name at that block number.
/// If not provided, the latest knowledge will be returned.
///
/// If receiving event in real-time, make sure to use `block` to give indexer
/// a cue to wait for the next block to respond.
#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct NamehashToNameRequest {
    pub hash: String,
    pub block: u64,
}

/// Response from `kns-indexer:kns-indexer:sys`.
#[derive(Debug, Serialize, Deserialize)]
pub enum IndexerResponses {
    /// Response to [`IndexerRequests::NamehashToName`].
    Name(Option<String>),
}

/// Update type used to convert kimap entries into node identities.
/// Only currently used in userspace for `eth:distro:sys` configuration.
#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct KnsUpdate {
    pub name: String,
    pub public_key: String,
    pub ips: Vec<String>,
    pub ports: BTreeMap<String, u16>,
    pub routers: Vec<String>,
}

impl KnsUpdate {
    pub fn get_protocol_port(&self, protocol: &str) -> u16 {
        self.ports.get(protocol).cloned().unwrap_or(0)
    }
}

/// Sign a message with the node's networking key. This may be used to prove
/// identity to other parties outside of using the networking protocol.
///
/// Note that the given message will be prepended with the source [`Address`]
/// of this message. This is done in order to not allow different processes
/// on the same node to sign messages for/as one another. The receiver of
/// the signed message should use [`verify()`] to verify the signature, which
/// takes a `from` address to match against that prepended signing [`Address`].
///
/// This function uses a 30-second timeout to reach `net:distro:sys`. If more
/// control over the timeout is needed, create a [`Request`] directly.
pub fn sign<T>(message: T) -> Result<Vec<u8>, SendError>
where
    T: Into<Vec<u8>>,
{
    Request::to(("our", "net", "distro", "sys"))
        .body(rmp_serde::to_vec(&NetAction::Sign).unwrap())
        .blob_bytes(message.into())
        .send_and_await_response(30)
        .unwrap()
        .map(|_resp| get_blob().unwrap().bytes)
}

/// Verify a signature on a message.
///
/// The receiver of a signature created using [`sign`] should use this function
/// to verify the signature, which takes a `from` address to match against
/// the prepended signing [`Address`] of the source process.
///
/// This function uses a 30-second timeout to reach `net:distro:sys`. If more
/// control over the timeout is needed, create a [`Request`] directly.
pub fn verify<T, U, V>(from: T, message: U, signature: V) -> Result<bool, SendError>
where
    T: Into<Address>,
    U: Into<Vec<u8>>,
    V: Into<Vec<u8>>,
{
    Request::to(("our", "net", "distro", "sys"))
        .body(
            rmp_serde::to_vec(&NetAction::Verify {
                from: from.into(),
                signature: signature.into(),
            })
            .unwrap(),
        )
        .blob_bytes(message.into())
        .send_and_await_response(30)
        .unwrap()
        .map(|resp| {
            let Ok(NetResponse::Verified(valid)) =
                rmp_serde::from_slice::<NetResponse>(resp.body())
            else {
                return false;
            };
            valid
        })
}

/// Get a [`crate::kimap::Kimap`] entry name from its namehash.
///
/// Default timeout is 30 seconds. Note that the responsiveness of the indexer
/// will depend on the block option used. The indexer will wait until it has
/// seen the block given to respond.
pub fn get_name<T>(namehash: T, block: Option<u64>, timeout: Option<u64>) -> Option<String>
where
    T: Into<String>,
{
    let res = Request::to(("our", "kns-indexer", "kns-indexer", "sys"))
        .body(
            serde_json::to_vec(&IndexerRequests::NamehashToName(NamehashToNameRequest {
                hash: namehash.into(),
                block: block.unwrap_or(0),
            }))
            .unwrap(),
        )
        .send_and_await_response(timeout.unwrap_or(30))
        .unwrap()
        .ok()?;

    let Ok(IndexerResponses::Name(maybe_name)) =
        serde_json::from_slice::<IndexerResponses>(res.body())
    else {
        return None;
    };

    maybe_name
}
