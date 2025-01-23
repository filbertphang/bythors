use crate::protocol::reliable_broadcast::packet::Packet;
use std::error::Error;
use std::fmt::Display;

// for RB, we send all packets via `Request`s, and acknowledge receiving a packet
// via a `Response`.`
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ProtocolRequest {
    pub packet: Packet,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum ProtocolResponse {
    Ack,
}

// may want to consider using a crate like `derive_more` to help us derive
// `Display` here.
impl Display for ProtocolRequest {
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
