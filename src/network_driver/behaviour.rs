use crate::network_driver::request_response::{ProtocolRequest, ProtocolResponse};
use libp2p::identity::Keypair;
use libp2p::request_response::ProtocolSupport;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{mdns, request_response, StreamProtocol};
use std::time::Duration;

// define a custom behaviour, aggregating:
// - mdns behaviour for peer discovery
// - request_response behaviour for sending messages
//   - cbor as serialization mechanism
//   - <ProtocolRequest, ProtocolResponse> as the request and response type respectively
#[derive(NetworkBehaviour)]
pub struct ProtocolBehaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub request_response: request_response::cbor::Behaviour<ProtocolRequest, ProtocolResponse>,
}

impl ProtocolBehaviour {
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
            request_response:
                request_response::cbor::Behaviour::<ProtocolRequest, ProtocolResponse>::new(
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
