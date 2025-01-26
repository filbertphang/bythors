use crate::protocol::{Message, Packet};

use std::error::Error;
use std::fmt::Display;

// for RB, we send all packets via `Request`s, and acknowledge receiving a packet
// via a `Response`.`
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ProtocolRequest<M> {
    pub packet: Packet<M>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum ProtocolResponse {
    Ack,
}

// may want to consider using a crate like `derive_more` to help us derive
// `Display` here.
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
