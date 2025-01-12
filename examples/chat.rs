use bythors::network_driver::network::Network;
use libp2p::identity::{rsa, Keypair, PeerId, PublicKey};
use tokio::sync::mpsc;
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

#[tokio::main]
async fn main() {
    // require that at least 2 args are passed in:
    // first argument is the keypair (identity) of the current node
    // second argument is the public key of the leader node
    // all remaining arguments are the public keys of the other nodes in the network,
    // including the current node
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

    // implement simple callback
    let callback = |msg| println!("{msg}");

    // initialize network driver
    let (sender, receiver) = mpsc::unbounded_channel::<String>();
    let mut network = Network::initialize(
        identity,
        &other_peer_ids,
        leader_peer_id,
        callback,
        receiver,
    )
    .unwrap();

    // initiaize logger
    env_logger::init();

    // simulate chat room: wait on stdin, and broadcast messages received from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();
    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                sender.send(line.clone()).unwrap();
            }

            // poll the network driver, to process new connections and events.
            // TODO: maybe figure out how to run this in a separate thread or something
            // so that the user doesn't have to worry about integrating network polling
            // into their event loop
            Ok(()) = network.poll() => {}
        }
    }
}
