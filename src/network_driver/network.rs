use core::error::Error;
use futures::StreamExt;
use log::info;
use std::str::FromStr;
use std::time::Duration;
use tokio::select;
use tokio::sync::mpsc;

use crate::marshal::initialization::initialize_lean_environment;
use crate::network_driver::behaviour::{ProtocolBehaviour, ProtocolBehaviourEvent};
use crate::network_driver::request_response::{ProtocolRequest, ProtocolResponse};
use crate::protocol::packet::Packet;
use crate::protocol::protocol::{initialize_ReliableBroadcastConcrete, Protocol};
use libp2p::identity::Keypair;
use libp2p::request_response::ResponseChannel;
use libp2p::swarm::SwarmEvent;
use libp2p::{mdns, request_response, PeerId, Swarm};

pub struct Network {
    swarm: Swarm<ProtocolBehaviour>,
    protocol: Protocol,
    receiver: mpsc::UnboundedReceiver<String>,
    // TODO: refine the type of the callback.
    // this will probably capture some part of the client application's environment, so we might
    // have to use `Fn` or `FnMut` instead of `fn`.
    // TODO: use the callback for something
    callback: fn(String) -> (),
}

impl Network {
    pub fn initialize(
        identity: Keypair,
        all_peer_ids: &Vec<PeerId>,
        leader_peer_id: &PeerId,
        callback: fn(String) -> (),
        receiver: mpsc::UnboundedReceiver<String>,
    ) -> Result<(Self), Box<dyn Error>> {
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

        // construct list of all peers, including self
        let self_id = swarm.local_peer_id().to_string();
        let leader_id = leader_peer_id.to_string();
        let mut all_peers: Vec<String> = all_peer_ids
            .iter()
            .map(|peer_id| peer_id.to_string())
            .collect();

        let protocol = unsafe {
            // initialize lean environment
            initialize_lean_environment(initialize_ReliableBroadcastConcrete);

            // construct lean protocol
            Protocol::create(all_peers, self_id, leader_id)
        };

        // Tell the swarm to listen on all interfaces and a random, OS-assigned port.
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        // construct network
        let network = Self {
            swarm,
            protocol,
            receiver,
            callback,
        };

        Ok(network)
    }

    pub async fn poll(&mut self) -> Result<(), Box<dyn Error>> {
        select! {
            // handle a client broadcast
            Some(message) = self.receiver.recv() => {
                info!("broadcasting message");
                self.broadcast(message);
            }

            // handle a swarm event (poll the swarm)
            event = self.swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {address:?}"),

                // MDNS: new peer discovered
                SwarmEvent::Behaviour(ProtocolBehaviourEvent::Mdns(
                    mdns::Event::Discovered(list),
                )) => {
                    info!("new peer(s) discovered");
                    for (peer_id, _multiaddr) in list {
                        self.swarm.dial(peer_id)?;
                    }
                }

                // MDNS: peer expired
                SwarmEvent::Behaviour(ProtocolBehaviourEvent::Mdns(
                    mdns::Event::Expired(list),
                )) => {
                    for (_peer_id, _multiaddr) in list {
                        // TODO: determine what happens to the protocol when a peer expires and re-connects.
                        info!("peer(s) expired");
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
                    info!("request received");
                    self.handle_request(request, channel);
                }

                // Request-Response: received a response
                SwarmEvent::Behaviour(ProtocolBehaviourEvent::RequestResponse(
                    request_response::Event::Message {
                        peer: _,
                        message:
                            request_response::Message::Response {
                                request_id: _,
                                response: _,
                            },
                    },
                )) => {
                    info!("response received");
                    self.handle_response();
                }

                // Ignore all other events.
                _ => {}
            }
        }
        // TODO: figure out how to handle errors here
        Ok(())
    }

    fn get_address(&self) -> String {
        self.swarm.local_peer_id().to_string()
    }

    /// Handles a broadcast from the client.
    /// Begins a new round of consensus, starting with the clinet-supplied message.
    fn broadcast(&mut self, message: String) {
        let packets = unsafe { self.protocol.send_message(self.get_address(), message) };
        self.transmit(packets);
    }

    fn handle_request(
        &mut self,
        request: ProtocolRequest,
        channel: ResponseChannel<ProtocolResponse>,
    ) {
        // acknowledge the packet
        self.swarm
            .behaviour_mut()
            .request_response
            .send_response(channel, ProtocolResponse::Ack)
            .expect("should be able to ack a request");

        let packets_to_send = unsafe { self.protocol.handle_packet(request.packet) };
        self.transmit(packets_to_send);
    }

    fn handle_response(&mut self) {
        // TODO: implement a proper handling mechanism
        // currently, responses are just acknowledgements (`ProtocolResponse::Ack`) of requests,
        // so we don't really need to do anything with it.
    }

    /// Transmits a packet to all other nodes.
    fn transmit(&mut self, packets: Vec<Packet>) {
        packets
            .into_iter()
            .for_each(|packet| self.send_individual_packet(packet));
    }

    fn send_individual_packet(&mut self, packet: Packet) {
        let src_id =
            PeerId::from_str(packet.src.as_str()).expect("expected well-formed source address");
        let dst_id = PeerId::from_str(packet.dst.as_str())
            .expect("expected well-formed destination address");

        match src_id == dst_id {
            false => {
                // sending packet to external node
                self.swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&dst_id, ProtocolRequest { packet });
            }
            true => {
                // simulate sending packet to self.
                // libp2p does not allow nodes to send packets to themselves, so we simulate
                // this behaviour by manually calling the packet handler.
                let packets_to_send = unsafe { self.protocol.handle_packet(packet) };
                self.transmit(packets_to_send);

                // TODO: check consensus output here (why here?)
            }
        }
    }
}
