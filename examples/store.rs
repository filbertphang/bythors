use bythors::network::Network;
use bythors::protocol::raft::Raft;
use libp2p::identity::{rsa, Keypair, PeerId, PublicKey};
use log::info;
use tokio::{io, io::AsyncBufReadExt, select};

// key generation
// (keypair in pem): openssl genrsa -out private.pem 2048
// (keypair in pkcs8 as der): openssl pkcs8 -in private.pem -inform PEM -topk8 -out private.pk8 -outform DER -nocrypt
// (pubkey in x509 as der): openssl rsa -in private.pem -pubout -out public.der -outform DER

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
    let private_key_path = format!("keys/private{node_num}.pk8");
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

fn parse_command(raw_input: String) -> Option<Command> {
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

/// a simple distributed key-value store (for strings),
/// using the raft protocol
#[tokio::main]
async fn main() {
    env_logger::init();

    // use key 1 as master node
    // use keys 2 - 4 as replicas
    let total_nodes = 4;
    let all_nodes: Vec<PeerId> = (1..=total_nodes)
        .map(|i| format!("keys/public{i}.der"))
        .map(|pk_path| parse_public_key(&pk_path))
        .collect();

    // launch master node
    let local_set = tokio::task::LocalSet::new();
    let all_nodes_copy = all_nodes.clone();
    local_set.spawn_local(async move {
        let mut stdin = io::BufReader::new(io::stdin()).lines();
        let mut network = start_replica(1, &all_nodes_copy, true);
        network.start().await;

        info!("ready!");

        loop {
            select! {
                Ok(Some(line)) = stdin.next_line() => {
                    let command = parse_command(line.clone());
                    match command {
                        Some(Command::Get { key }) => {
                            let res = network.check_output(key.clone());
                            println!("GET key {key}: found {res:#?}");
                        },
                        Some(Command::Put { key, val }) => {
                            network.broadcast((key.clone(), val.clone()));
                            println!("PUT key: {key} value: {val:#?}");
                        },
                        None => println!("invalid command: {line}"),
                    }
                }

                // poll the network driver, to process new connections and events.
                _ = network.poll() => { }
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

            loop {
                network.poll().await
            }
        });
    }

    local_set.await
}
