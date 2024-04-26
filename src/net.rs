use crate::*;

//
// Networking protocol types
//

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub name: NodeId,
    pub networking_key: String,
    pub ws_routing: Option<(String, u16)>,
    pub allowed_routers: Vec<NodeId>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnsUpdate {
    pub name: String, // actual username / domain name
    pub owner: String,
    pub node: String, // hex namehash of node
    pub public_key: String,
    pub ips: Vec<String>,
    pub ports: HashMap<String, u16>,
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
