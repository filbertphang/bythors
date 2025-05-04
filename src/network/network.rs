use super::behaviour::{ProtocolBehaviour, ProtocolBehaviourEvent};
use super::request_response::{ProtocolRequest, ProtocolResponse};

use crate::marshal::initialization::initialize_lean_environment;
use crate::protocol::raft::RaftPersistentState;
use crate::protocol::{Message, Packet, Protocol};

use core::error::Error;
use futures::StreamExt;
use libp2p::identity::Keypair;
use libp2p::request_response::ResponseChannel;
use libp2p::swarm::SwarmEvent;
use libp2p::{mdns, request_response, PeerId, Swarm};
use log::{debug, info};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

pub enum NetworkPollResult {
    ProtocolEvent,
    OtherEvent,
}

/// Implements a network layer for the consensus protocol.
/// The network layer is responsible for communication between nodes to reach consensus.
pub struct Network<T>
where
    T: Protocol,
    T::Message: Message + 'static,
{
    swarm: Swarm<ProtocolBehaviour<T::Message>>,
    protocol: T,
    all_peers: Vec<String>,
    node_state_path: PathBuf,
}

impl<T> Network<T>
where
    T: Protocol,
    T::Message: Message + 'static,
{
    /// Creates and sets up a new network.
    /// Does not run anything yet.
    pub fn initialize(
        identity: Keypair,
        all_peer_ids: &Vec<PeerId>,
        leader_peer_id: &PeerId,
        init_lean: bool,
        node_state_path: PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        // For diagnostics (I never figured out how to use this)
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
        let all_peers: Vec<String> = all_peer_ids
            .iter()
            .map(|peer_id| peer_id.to_string())
            .collect();

        let protocol = unsafe {
            if init_lean {
                // initialize lean environment
                initialize_lean_environment(T::initialize_lean);
            }

            // construct lean protocol
            T::create(all_peers.clone(), self_id, leader_id)
        };

        // Tell the swarm to listen on all interfaces and a random, OS-assigned port.
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        // construct network
        let mut network = Self {
            swarm,
            protocol,
            all_peers,
            node_state_path,
        };

        // load state, if present
        network.load_node_state();

        Ok(network)
    }

    /// Starts the network.
    /// This connects to all nodes, and starts the protocol execution.
    pub async fn start(&mut self) {
        // wait for connections
        while self.swarm.connected_peers().count() != (self.all_peers.len() - 1) {
            let _ = self.poll().await;
        }

        // start protocol
        unsafe {
            let packets = self.protocol.start();
            self.transmit(packets);
        }
    }

    /// Polls the network for updates.
    /// Network events occur as specified by the NetworkBehaviour.
    /// We handle both MDNS events (for discovery) and
    /// request-response events (for node-to-node communication) here.
    pub async fn poll(&mut self) -> NetworkPollResult {
        // handle a swarm event (poll the swarm)
        let event = self.swarm.select_next_some().await;
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("listening on {address:?}");
                NetworkPollResult::OtherEvent
            }

            // MDNS: new peer discovered
            SwarmEvent::Behaviour(ProtocolBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                info!("new peer(s) discovered");
                for (peer_id, _multiaddr) in list {
                    // ignoring the result of this, because sometimes it would error
                    // when a peer expires, is re-discovered, and we try to dial it
                    // when it's already dialed.
                    let _ = self.swarm.dial(peer_id);
                }
                NetworkPollResult::OtherEvent
            }

            // MDNS: peer expired
            SwarmEvent::Behaviour(ProtocolBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                for (_peer_id, _multiaddr) in list {
                    // TODO: determine what happens to the protocol when a peer expires and re-connects.
                    info!("peer(s) expired");
                }
                NetworkPollResult::OtherEvent
            }

            // Request-Response: received a request
            SwarmEvent::Behaviour(ProtocolBehaviourEvent::RequestResponse(
                request_response::Event::Message {
                    message:
                        request_response::Message::Request {
                            request, channel, ..
                        },
                    ..
                },
            )) => {
                self.handle_request(request, channel);
                NetworkPollResult::ProtocolEvent
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
                self.handle_response();
                NetworkPollResult::OtherEvent
            }

            // Ignore all other events.
            _ => NetworkPollResult::OtherEvent,
        }
    }

    /// Gets the address of this node.
    fn get_address(&self) -> String {
        self.swarm.local_peer_id().to_string()
    }

    /// Handles a broadcast from the client.
    /// Begins a new round of consensus, starting with the client-supplied message.
    // TODO: add `round` as a parameter to this, and throw suitable errors if we're trying to
    // broadcast to an existing round (ongoing or concluded)
    pub fn broadcast(&mut self, message: (String, String)) {
        let packets = unsafe { self.protocol.start_round(self.get_address(), message) };
        self.transmit(packets);
    }

    /// Handles an incoming packet from a node.
    fn handle_request(
        &mut self,
        request: ProtocolRequest<T::Message>,
        channel: ResponseChannel<ProtocolResponse>,
    ) {
        info!("request received");
        let packet = request.packet;

        // acknowledge the packet
        self.swarm
            .behaviour_mut()
            .request_response
            .send_response(channel, ProtocolResponse::Ack)
            .expect("should be able to ack a request");

        let packets_to_send = unsafe { self.protocol.handle_packet(packet) };

        // persist current state to disk before responding
        self.persist_node_state();

        self.transmit(packets_to_send);
    }

    /// Handles an acknowledgement that a packet was received.
    fn handle_response(&mut self) {
        // TODO: implement a proper handling mechanism
        // currently, responses are just acknowledgements (`ProtocolResponse::Ack`) of requests,
        // so we don't really need to do anything with it.
        info!("response received");
    }

    /// Transmits a sequence of packets to all other nodes.
    fn transmit(&mut self, packets: Vec<Packet<T::Message>>) {
        packets
            .into_iter()
            .for_each(|packet| self.send_individual_packet(packet));
    }

    /// Sends a single packet to all other nodes.
    fn send_individual_packet(&mut self, packet: Packet<T::Message>) {
        info!("sending packet");
        debug!("packet contents: {packet:#?}");
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
            }
        }
    }

    /// Checks the stored value for the given key.
    /// Used only for GET requests.
    pub fn check_output(&mut self, key: String) -> Option<String> {
        unsafe { self.protocol.check_output(key) }
    }

    /// Handle a timeout.
    /// This occurs when a follower node has not received a heartbeat from
    /// a leader for some time.
    pub fn timeout(&mut self) {
        info!("getting timed out (if follower)");
        let packets_to_send = unsafe { self.protocol.handle_timeout() };
        self.transmit(packets_to_send);
    }

    /// Sends a heartbeat.
    /// The leader node must send heartbeats to follower nodes periodicially.
    pub fn send_heartbeat(&mut self) {
        info!("sending heartbeat (if leader)");
        let packets_to_send = unsafe { self.protocol.send_heartbeat() };
        self.transmit(packets_to_send);
    }

    /// Checks if a node is the leader node.
    pub fn is_leader(&mut self) -> bool {
        unsafe { self.protocol.is_leader() }
    }

    /// Writes the node state to disk.
    /// This is necessary for crash tolerance.
    pub fn persist_node_state(&self) {
        let persistent_state = unsafe { self.protocol.get_persistent_state() };
        persistent_state.to_disk(&self.node_state_path);
    }

    /// Loads the node state from disk.
    /// This is necessary for crash tolerance.
    pub fn load_node_state(&mut self) {
        if let Ok(true) = fs::exists(&self.node_state_path) {
            unsafe {
                let persistent_state = RaftPersistentState::from_disk(&self.node_state_path);
                self.protocol.load_persistent_state(persistent_state);
            }
        }
    }
}
