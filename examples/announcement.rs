use bythors::network_driver::network::Network;
use libp2p::identity::{rsa, Keypair, PeerId, PublicKey};
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

/// Initializes an announcement system, where announcement messages can be broadcast
/// from the leader node to all other recipient nodes.
#[tokio::main]
async fn main() {
    // require that at least 2 args are passed in:
    // first argument is the keypair (identity) of the current node
    // all remaining arguments are the public keys of ALL nodes in the network, including
    // the current node.
    // the FIRST of these public keys (i.e. the second argument) will be used as the protocol leader.
    // TODO: figure out a better/less confusing way to handle pubkeys
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() >= 3);

    // parse identity keypair for current node
    let mut identity_raw = std::fs::read(&args[1]).unwrap();
    let identity = Keypair::rsa_from_pkcs8(&mut identity_raw).unwrap();

    // parse public keys for other nodes
    let other_peer_ids: Vec<PeerId> = args
        .iter()
        .skip(2)
        .map(|path| parse_public_key(path))
        .collect();
    let leader_peer_id = &other_peer_ids[0];

    let mut leader_short_id = leader_peer_id.to_string();
    leader_short_id.truncate(6);

    // implement simple callback
    let callback = |msg, round| println!("(round {round}) {msg}");

    // initialize network driver
    let mut network =
        Network::initialize(identity, &other_peer_ids, leader_peer_id, callback).unwrap();

    // initiaize logger
    env_logger::init();

    // simulate announcement channel: wait on stdin, and broadcast messages received from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();
    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                let message = format!{"{}: {}", leader_short_id, line};
                network.broadcast(message);
            }

            // poll the network driver, to process new connections and events.
            Ok(()) = network.poll() => {
                // TODO: maybe run this in a separate thread or something
                // so that the user doesn't have to worry about integrating network polling
                // into their event loop
            }
        }
    }
}
