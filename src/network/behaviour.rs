use super::request_response::{ProtocolRequest, ProtocolResponse};

use crate::protocol::Message;

use libp2p::identity::Keypair;
use libp2p::request_response::ProtocolSupport;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{mdns, request_response, StreamProtocol};
use std::time::Duration;

// define a custom behaviour, aggregating:
// - mdns behaviour for peer discovery
// - request_response behaviour for sending messages
//   - cbor as serialization mechanism
//   - <ProtocolRequest<P>, ProtocolResponse> as the request and response type respectively
//     (P is the packet type)
//
// note: trait bound for Message on the struct definitioon is mandatory here due to the implementation
// of cbor::Behaviour.
#[derive(NetworkBehaviour)]
pub struct ProtocolBehaviour<M>
where
    M: Message + 'static,
{
    pub mdns: mdns::tokio::Behaviour,
    pub request_response: request_response::cbor::Behaviour<ProtocolRequest<M>, ProtocolResponse>,
}

impl<M> ProtocolBehaviour<M>
where
    M: Message,
{
    pub fn new(keypair: &Keypair) -> Self {
        let local_peer_id = keypair.public().to_peer_id();

        // TODO: add customizable config for MDNS and request_response.
        let mdns_config = mdns::Config {
            ttl: Duration::from_secs(30),
            query_interval: Duration::from_secs(5),
            enable_ipv6: false,
        };

        Self {
            mdns: mdns::tokio::Behaviour::new(mdns_config, local_peer_id).unwrap(),
            request_response: request_response::cbor::Behaviour::<
                ProtocolRequest<M>,
                ProtocolResponse,
            >::new(
                [(
                    StreamProtocol::new("/bythors/protocol/1"),
                    ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            ),
        }
    }
}

// TODO: add swarm initializer and network handlers (send message, handle message, etc)
// refer to `libp2p_rb.rs`
