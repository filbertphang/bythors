use crate::network::{Network, NetworkPollResult};
use crate::protocol::raft::Raft;
use crate::store::{heartbeat, shutdown};
use libp2p::identity::{rsa, Keypair, PeerId, PublicKey};
use log::info;
use std::net::SocketAddr;
use tokio::io::AsyncBufReadExt;
use tokio::net::TcpListener;
use tokio::{io, select};

// intended to be ran from crate base directory
const BASE_DIR: &str = "keys";

fn parse_public_key(path: &str) -> PeerId {
    let public_key_raw = std::fs::read(path).unwrap();
    let public_key: PublicKey = rsa::PublicKey::try_decode_x509(public_key_raw.as_slice())
        .unwrap()
        .into();
    let peer_id = public_key.into();
    peer_id
}

fn start_replica(node_num: usize, all_nodes: &Vec<PeerId>, init_lean: bool) -> Network<Raft> {
    assert!(!all_nodes.is_empty());

    // parse identity keypair for current node
    let private_key_path = format!("{BASE_DIR}/private{node_num}.pk8");
    let mut identity_raw = std::fs::read(private_key_path).unwrap();
    let identity = Keypair::rsa_from_pkcs8(&mut identity_raw).unwrap();

    // init network
    let network: Network<Raft> =
        Network::initialize(identity, all_nodes, &all_nodes[0], init_lean).unwrap();
    network
}

// TODO:
// one node per process
// multiple task?? for handling tcp connections (can be on same thread)
// get multiple tcp connections, each connection just pushes the shit onto a mpsc channel? read-only
// ^ this can live in its own thread
// in the main thread, we poll for a new message, and process it, and write to idk another channel maybe
// nask jdhaskjhdksahjsakjhsakhsakjhaskjh

/// a simple distributed key-value store (for strings),
/// using the raft protocol
#[tokio::main]
pub async fn main() {
    env_logger::init();
    const ELECTION_TIMEOUT: u64 = 10000;
    const HEARTBEAT_INTERVAL: u64 = ELECTION_TIMEOUT / 4;

    // use key 1 as master node
    // use keys 2 - 4 as replicas
    let total_nodes = 4;
    let all_nodes: Vec<PeerId> = (1..=total_nodes)
        .map(|i| format!("{BASE_DIR}/public{i}.der"))
        .map(|pk_path| parse_public_key(&pk_path))
        .collect();

    // create shutdown receivers
    let (shutdown_bcast, _) = tokio::sync::broadcast::channel(2);

    // launch master node
    let local_set = tokio::task::LocalSet::new();
    let all_nodes_copy = all_nodes.clone();

    let shutdown_bcast_master = shutdown_bcast.clone();
    local_set.spawn_local(async move {
        // spawn libp2p network (for consensus)
        let mut stdin = io::BufReader::new(io::stdin()).lines();
        let mut network = start_replica(1, &all_nodes_copy, true);
        network.start().await;

        // set up tcp listener to handle client connections
        let host = "127.0.0.1";
        let port = "8080";
        let addr = format!("{host}:{port}");
        let tcp_listener = TcpListener::bind(addr)
            .await
            .expect("should be able to create tcp listener");
        // let mut cons = HashMap<SocketAddr,

        info!("master node ready!");
        // Future that indicates when a heartbeat has not been received for some time
        let heartbeat_timeout = heartbeat::new_random(ELECTION_TIMEOUT);
        tokio::pin!(heartbeat_timeout);

        // Future that is resolved whenever a leader should send a heartbeat
        let should_send_heartbeat = heartbeat::new_random(HEARTBEAT_INTERVAL);
        tokio::pin!(should_send_heartbeat);

        loop {
            select! {
                // handle stdin (only for quitting)
                Ok(Some(line)) = stdin.next_line() => {
                    if line == "q" {
                        println!("quitting");
                        // handle shutdown
                        let _ = shutdown_bcast_master.send(());
                        return;
                    } else {
                        println!("unrecognised: {line}");
                    }
                    // let command = parse_command(line.clone());
                    // match command {
                    //     Some(Command::Get { key }) => {
                    //         let res = network.check_output(key.clone());
                    //         println!("GET key {key}: found {res:?}");
                    //     },
                    //     Some(Command::Put { key, val }) => {
                    //         network.broadcast((key.clone(), val.clone()));
                    //         println!("PUT key: {key} value: {val:#?}");
                    //     },
                    //     None => println!("invalid command: {line}"),
                    // }
                }

                // heartbeat not received: send timeout
                () = &mut heartbeat_timeout => {
                    network.timeout();
                }

                // time to send a heartbeat
                () = &mut should_send_heartbeat => {
                    network.send_heartbeat();
                    // reset the timer for the next one
                    heartbeat::reset(should_send_heartbeat.as_mut(), HEARTBEAT_INTERVAL);
                }

                // poll the network driver, to process new connections and events.
                res = network.poll() => {
                    match res {
                        NetworkPollResult::ProtocolEvent => {
                            // reset the heartbeat timer
                            heartbeat::reset(heartbeat_timeout.as_mut(), ELECTION_TIMEOUT);
                        }
                        NetworkPollResult::OtherEvent => {}
                    }
                }
            }
        }
    });

    // launch replicas
    for i in 2..=total_nodes {
        // using `spawn_local` because otherwise, Network<T> must be Send
        // (due to tokio::spawn requirements)
        // probably not a good idea to share FFI stuff between threads, though
        // i'm not too sure how that interacts (e.g. will FFI pointers still be valid?)
        let all_nodes_copy = all_nodes.clone();
        let shutdown_recv = shutdown_bcast.subscribe();
        let mut shutdown = shutdown::Shutdown::new(shutdown_recv);

        local_set.spawn_local(async move {
            // lean can only be initialized once per process (else we segfault)
            // so, we skip initialzing in the replica threads
            let mut network = start_replica(i, &all_nodes_copy, false);
            network.start().await;

            info!("replica {i} ready!");
            // Future that indicates when a heartbeat has not been received for some time
            let heartbeat_timeout = heartbeat::new_random(ELECTION_TIMEOUT);
            tokio::pin!(heartbeat_timeout);

            // Future that is resolved whenever a leader should send a heartbeat
            let should_send_heartbeat = heartbeat::new_random(HEARTBEAT_INTERVAL);
            tokio::pin!(should_send_heartbeat);

            loop {
                select! {
                    // server shutdown
                    () = shutdown.recv() => { return }

                    // heartbeat not received: send timeout
                    () = &mut heartbeat_timeout => {
                        network.timeout();
                    }

                    // time to send a heartbeat
                    () = &mut should_send_heartbeat => {
                        network.send_heartbeat();
                        // reset the timer for the next one
                        heartbeat::reset(should_send_heartbeat.as_mut(), HEARTBEAT_INTERVAL);
                    }

                    // poll the network driver, to process new connections and events.
                    res = network.poll() => {
                        match res {
                            NetworkPollResult::ProtocolEvent => {
                                // reset the heartbeat timer
                                heartbeat::reset(heartbeat_timeout.as_mut(), ELECTION_TIMEOUT);
                            }
                            NetworkPollResult::OtherEvent => {}
                        }
                    }
                }
            }
        });
    }

    local_set.await
}
