use core::error::Error;
use futures::StreamExt;
use std::time::Duration;
use tokio::select;
use tokio::sync::mpsc;

use crate::marshal::initialization::initialize_lean_environment;
use crate::network_driver::behaviour::{ProtocolBehaviour, ProtocolBehaviourEvent};
use crate::protocol::protocol::{initialize_ReliableBroadcastConcrete, Protocol};
use libp2p::identity::Keypair;
use libp2p::request_response::{ProtocolSupport, ResponseChannel};
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{mdns, request_response, PeerId, StreamProtocol, Swarm};

struct Network {
    swarm: Swarm<ProtocolBehaviour>,
    protocol: Protocol,
    receiver: mpsc::UnboundedReceiver<String>,
    // TODO: refine the type of the callback.
    // this will probably capture some part of the client application's environment, so we might
    // have to use `Fn` or `FnMut` instead of `fn`.
    callback: fn(String) -> (),
}

impl Network {
    pub fn initialize(
        identity: Keypair,
        other_peer_ids: Vec<&PeerId>,
        leader_peer_id: &PeerId,
        callback: fn(String) -> (),
    ) -> Result<(Self, mpsc::UnboundedSender<String>), Box<dyn Error>> {
        // for diagnostics
        // tracing_subscriber::fmt()
        //     .with_env_filter(EnvFilter::from_default_env())
        //     .init();

        // set up p2p network
        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(identity)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::tls::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_behaviour(|keypair| ProtocolBehaviour::new(keypair))?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
            })
            .build();

        // set up communication channel between client and network
        let (sender, receiver) = mpsc::unbounded_channel::<String>();

        // construct list of all peers, including self
        let self_id = swarm.local_peer_id().to_string();
        let leader_id = leader_peer_id.to_string();
        let mut all_peers: Vec<String> = other_peer_ids
            .iter()
            .map(|peer_id| peer_id.to_string())
            .collect();
        all_peers.push(self_id.clone());

        let protocol = unsafe {
            // initialize lean environment
            initialize_lean_environment(initialize_ReliableBroadcastConcrete);

            // construct lean protocol
            Protocol::create(all_peers, self_id, leader_id)
        };

        // construct network
        let network = Self {
            swarm,
            protocol,
            receiver,
            callback,
        };

        Ok((network, sender))
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        // Tell the swarm to listen on all interfaces and a random, OS-assigned port.
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        loop {
            select! {
                    // handle a client broadcast
                    Some(broadcast_msg) = self.receiver.recv() => {
                        // TODO: broadcast message
                        self.broadcast();
                    }

                    // handle a swarm event (poll the swarm)
                    event = self.swarm.select_next_some() => match event {
                        SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {address:?}"),

                        // MDNS: new peer discovered
                        SwarmEvent::Behaviour(ProtocolBehaviourEvent::Mdns(
                            mdns::Event::Discovered(list),
                        )) => {
                            for (peer_id, _multiaddr) in list {
                                self.swarm.dial(peer_id)?;
                            }
                        }

                        // MDNS: peer expired
                        SwarmEvent::Behaviour(ProtocolBehaviourEvent::Mdns(
                            mdns::Event::Expired(list),
                        )) => {
                            for (peer_id, _multiaddr) in list {
                                // TODO: determine what happens to the protocol when a peer expires and re-connects.
                            }
                        }

                        // Request-Response: received a request
                        SwarmEvent::Behaviour(ProtocolBehaviourEvent::RequestResponse(
                            request_response::Event::Message {
                                message:
                                    request_response::Message::Request {
                                        request,
                                        channel,
                                        ..
                                    },
                                    ..
                            },
                        )) => {
                            // TODO: handle request
                            self.handle_request();
                        }

                        // Request-Response: received a response
                        SwarmEvent::Behaviour(ProtocolBehaviourEvent::RequestResponse(
                            request_response::Event::Message {
                                peer,
                                message:
                                    request_response::Message::Response {
                                        request_id: _,
                                        response,
                                    },
                            },
                        )) => {
                            // TODO: handle response
                            self.handle_response();
                        }

                        // Ignore all other events.
                        _ => {}
                    }



            }
        }
    }

    fn broadcast(&mut self) {
        // nyi
    }

    fn handle_request(&mut self) {
        // nyi
    }

    fn handle_response(&mut self) {
        // nyi
    }
}
