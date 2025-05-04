use crate::protocol::{Message, Packet};

use std::error::Error;
use std::fmt::Display;

/// Node-to-node communication is facilitated by the RequestResponse network behaviour.
///
/// A node sends a Packet<M> to another node over the network via a ProtocolRequest<M>.
///
/// The recipient node, upon receiving the ProtoclRequest<M>, acknowledges that it has received
/// the node by replying with a ProtocolResponse.
///
/// The recieved Packet<M> is then forwarded to the node's protocol implementation, and response
/// packets/messages are generated.
///
/// Response Packet<M>s are then sent via ProtocolRequest<M>s, as above, and the cycle repeats.

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ProtocolRequest<M> {
    pub packet: Packet<M>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum ProtocolResponse {
    Ack,
}

// TODO: may want to consider using a crate like `derive_more` to help derive `Display` here.
impl<M> Display for ProtocolRequest<M>
where
    M: Display + Message,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "request: {}", self.packet)
    }
}

impl Display for ProtocolResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "response: ack")
    }
}

impl Error for ProtocolResponse {}
