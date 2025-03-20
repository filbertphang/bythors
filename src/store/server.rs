use crate::network::{Network, NetworkPollResult};
use crate::protocol::raft::Raft;
use libp2p::identity::{rsa, Keypair, PeerId, PublicKey};
use log::info;
use std::pin::Pin;
use std::time::Duration;
use tokio::select;
use tokio::time::{Instant, Sleep};

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

enum Command {
    Get { key: String },
    Put { key: String, val: String },
}

fn parse_commandg(raw_input: String) -> Option<Command> {
    let res: Vec<&str> = raw_input.splitn(3, ' ').collect();

    if res.len() < 2 {
        return None;
    }

    let command = res[0].trim().to_uppercase();
    match command.as_str() {
        "GET" => Some(res).filter(|v| v.len() == 2).map(|v| Command::Get {
            key: v[1].trim().to_string(),
        }),
        "PUT" => Some(res).filter(|v| v.len() == 3).map(|v| Command::Put {
            key: v[1].trim().to_string(),
            val: v[2].trim().to_string(),
        }),
        _ => None,
    }
}

fn randomize_timeout(timeout: u64) -> Duration {
    Duration::from_millis(rand::random_range(timeout..=(2 * timeout)))
}

fn reset_heartbeat(heartbeat: Pin<&mut Sleep>, timeout: u64) {
    let timeout_duration = randomize_timeout(timeout);
    let deadline = Instant::now()
        .checked_add(timeout_duration)
        .expect("should be able to create new deadline");
    heartbeat.reset(deadline);
}

#[rocket::get("/<key>")]
fn dummy(key: &str) -> String {
    format!("hello, {key}")
}

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

    // launch master node
    let local_set = tokio::task::LocalSet::new();
    let all_nodes_copy = all_nodes.clone();
    local_set.spawn_local(async move {
        // spawn libp2p network (for consensus)
        // let mut stdin = io::BufReader::new(io::stdin()).lines();
        let mut network = start_replica(1, &all_nodes_copy, true);
        network.start().await;

        // start server, to serve http requests
        let _rocket = rocket::build()
            .mount("/v2/keys", rocket::routes![dummy])
            .launch()
            .await;

        info!("master node ready!");
        // Future that indicates when a heartbeat has not been received for some time
        let heartbeat_timeout = tokio::time::sleep(randomize_timeout(ELECTION_TIMEOUT));
        tokio::pin!(heartbeat_timeout);

        // Future that is resolved whenever a leader should send a heartbeat
        let should_send_heartbeat = tokio::time::sleep(randomize_timeout(HEARTBEAT_INTERVAL));
        tokio::pin!(should_send_heartbeat);

        loop {
            select! {
                // handle stdin
                // Ok(Some(line)) = stdin.next_line() => {
                //     let command = parse_command(line.clone());
                //     match command {
                //         Some(Command::Get { key }) => {
                //             let res = network.check_output(key.clone());
                //             println!("GET key {key}: found {res:?}");
                //         },
                //         Some(Command::Put { key, val }) => {
                //             network.broadcast((key.clone(), val.clone()));
                //             println!("PUT key: {key} value: {val:#?}");
                //         },
                //         None => println!("invalid command: {line}"),
                //     }
                // }

                // heartbeat not received: send timeout
                () = &mut heartbeat_timeout => {
                    network.timeout();
                }

                // time to send a heartbeat
                () = &mut should_send_heartbeat => {
                    network.send_heartbeat();
                    // reset the timer for the next one
                    reset_heartbeat(should_send_heartbeat.as_mut(), HEARTBEAT_INTERVAL);
                }

                // poll the network driver, to process new connections and events.
                res = network.poll() => {
                    match res {
                        NetworkPollResult::ProtocolEvent => {
                            // reset the heartbeat timer
                            reset_heartbeat(heartbeat_timeout.as_mut(), ELECTION_TIMEOUT);
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

        local_set.spawn_local(async move {
            // lean can only be initialized once per process (else we segfault)
            // so, we skip initialzing in the replica threads
            let mut network = start_replica(i, &all_nodes_copy, false);
            network.start().await;

            info!("replica {i} ready!");
            // Future that indicates when a heartbeat has not been received for some time
            let heartbeat_timeout = tokio::time::sleep(randomize_timeout(ELECTION_TIMEOUT));
            tokio::pin!(heartbeat_timeout);

            // Future that is resolved whenever a leader should send a heartbeat
            let should_send_heartbeat = tokio::time::sleep(randomize_timeout(HEARTBEAT_INTERVAL));
            tokio::pin!(should_send_heartbeat);

            loop {
                select! {
                    // heartbeat not received: send timeout
                    () = &mut heartbeat_timeout => {
                        network.timeout();
                    }

                    // time to send a heartbeat
                    () = &mut should_send_heartbeat => {
                        network.send_heartbeat();
                        // reset the timer for the next one
                        reset_heartbeat(should_send_heartbeat.as_mut(), HEARTBEAT_INTERVAL);
                    }

                    // poll the network driver, to process new connections and events.
                    res = network.poll() => {
                        match res {
                            NetworkPollResult::ProtocolEvent => {
                                // reset the heartbeat timer
                                reset_heartbeat(heartbeat_timeout.as_mut(), ELECTION_TIMEOUT);
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
