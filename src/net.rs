use crate::{get_blob, Address, NodeId, Request, SendError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

//
// Networking protocol types
//

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub name: NodeId,
    pub networking_key: String,
    pub routing: NodeRouting,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NodeRouting {
    Routers(Vec<NodeId>),
    Direct {
        ip: String,
        ports: BTreeMap<String, u16>,
    },
}

impl Identity {
    pub fn is_direct(&self) -> bool {
        matches!(&self.routing, NodeRouting::Direct { .. })
    }
    pub fn get_protocol_port(&self, protocol: &str) -> Option<u16> {
        match &self.routing {
            NodeRouting::Routers(_) => None,
            NodeRouting::Direct { ports, .. } => ports.get(protocol).cloned(),
        }
    }
    pub fn routers(&self) -> Option<&Vec<NodeId>> {
        match &self.routing {
            NodeRouting::Routers(routers) => Some(routers),
            NodeRouting::Direct { .. } => None,
        }
    }
}

/// Must be parsed from message pack vector.
/// all Get actions must be sent from local process. used for debugging
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetAction {
    /// Received from a router of ours when they have a new pending passthrough for us.
    /// We should respond (if we desire) by using them to initialize a routed connection
    /// with the NodeId given.
    ConnectionRequest(NodeId),
    /// can only receive from trusted source, for now just ourselves locally,
    /// in the future could get from remote provider
    KnsUpdate(KnsUpdate),
    KnsBatchUpdate(Vec<KnsUpdate>),
    /// get a list of peers we are connected to
    GetPeers,
    /// get the [`Identity`] struct for a single peer
    GetPeer(String),
    /// get the [`NodeId`] associated with a given namehash, if any
    GetName(String),
    /// get a user-readable diagnostics string containing networking inforamtion
    GetDiagnostics,
    /// sign the attached blob payload, sign with our node's networking key.
    /// **only accepted from our own node**
    /// **the source [`Address`] will always be prepended to the payload**
    Sign,
    /// given a message in blob payload, verify the message is signed by
    /// the given source. if the signer is not in our representation of
    /// the PKI, will not verify.
    /// **the `from` [`Address`] will always be prepended to the payload**
    Verify {
        from: Address,
        signature: Vec<u8>,
    },
}

/// For now, only sent in response to a ConnectionRequest.
/// Must be parsed from message pack vector
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetResponse {
    Accepted(NodeId),
    Rejected(NodeId),
    /// response to [`NetAction::GetPeers`]
    Peers(Vec<Identity>),
    /// response to [`NetAction::GetPeer`]
    Peer(Option<Identity>),
    /// response to [`NetAction::GetName`]
    Name(Option<String>),
    /// response to [`NetAction::GetDiagnostics`]. a user-readable string.
    Diagnostics(String),
    /// response to [`NetAction::Sign`]. contains the signature in blob
    Signed,
    /// response to [`NetAction::Verify`]. boolean indicates whether
    /// the signature was valid or not. note that if the signer node
    /// cannot be found in our representation of PKI, this will return false,
    /// because we cannot find the networking public key to verify with.
    Verified(bool),
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct KnsUpdate {
    pub name: String, // actual username / domain name
    pub owner: String,
    pub node: String, // hex namehash of node
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

//
// Helpers
//

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

/// get a kimap name from namehash
pub fn get_name(namehash: &str, timeout: Option<u64>) -> anyhow::Result<String> {
    let res = Request::to(("our", "net", "distro", "sys"))
        .body(rmp_serde::to_vec(&NetAction::GetName(namehash.to_string())).unwrap())
        .send_and_await_response(timeout.unwrap_or(5))??;

    let response = rmp_serde::from_slice::<NetResponse>(res.body())?;
    if let NetResponse::Name(name) = response {
        // is returning an option optimal?
        // getting an error for send/malformatted hash/not found seems better
        if let Some(name) = name {
            return Ok(name);
        } else {
            return Err(anyhow::anyhow!("name not found"));
        }
    } else {
        Err(anyhow::anyhow!("unexpected response: {:?}", response))
    }
}

/// take a DNSwire-formatted node ID from chain and convert it to a String
pub fn dnswire_decode(wire_format_bytes: &[u8]) -> Result<String, DnsDecodeError> {
    let mut i = 0;
    let mut result = Vec::new();

    while i < wire_format_bytes.len() {
        let len = wire_format_bytes[i] as usize;
        if len == 0 {
            break;
        }
        let end = i + len + 1;
        let mut span = match wire_format_bytes.get(i + 1..end) {
            Some(span) => span.to_vec(),
            None => return Err(DnsDecodeError::FormatError),
        };
        span.push('.' as u8);
        result.push(span);
        i = end;
    }

    let flat: Vec<_> = result.into_iter().flatten().collect();

    let name = String::from_utf8(flat).map_err(|e| DnsDecodeError::Utf8Error(e))?;

    // Remove the trailing '.' if it exists (it should always exist)
    if name.ends_with('.') {
        Ok(name[0..name.len() - 1].to_string())
    } else {
        Ok(name)
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum DnsDecodeError {
    Utf8Error(std::string::FromUtf8Error),
    FormatError,
}

impl std::fmt::Display for DnsDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DnsDecodeError::Utf8Error(e) => write!(f, "UTF-8 error: {}", e),
            DnsDecodeError::FormatError => write!(f, "Format error"),
        }
    }
}
