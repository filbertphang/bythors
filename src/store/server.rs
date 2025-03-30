use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::network::{Network, NetworkPollResult};
use crate::protocol::raft::Raft;
use crate::store::{command, heartbeat, shutdown, socket};

use clap::Parser;
use libp2p::identity::{rsa, Keypair, PeerId, PublicKey};
use log::{debug, info};
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::{io, select};

const BCAST_CHANNEL_CAPACITY: usize = 1024;

#[derive(Parser, Clone)]
struct Args {
    node_number: usize,
    total_nodes: usize,

    #[arg(default_value_t = 10000)]
    election_timeout: u64,

    #[arg(default_value_t = 2500)]
    heartbeat_interval: u64,

    /// dir where keypairs are stored
    // (keypairs are only used to generate identities for each node)
    #[arg(default_value_t = String::from("keys"))]
    keys_dir: String,

    /// dir to store server state
    data_dir: Option<String>,

    /// clear and overwrite existing server state, instead of resuming
    clear_data: bool,
}

fn parse_public_key(path: &str) -> PeerId {
    let public_key_raw =
        std::fs::read(path).expect(&format!("should be able to read file at {path}"));
    let public_key: PublicKey = rsa::PublicKey::try_decode_x509(public_key_raw.as_slice())
        .unwrap()
        .into();
    let peer_id = public_key.into();
    peer_id
}

async fn start_node(
    args: Args,
    all_nodes: &Vec<PeerId>,
    shutdown_rx: broadcast::Receiver<()>,
    mut request_rx: broadcast::Receiver<(command::Request, SocketAddr)>,
    write_streams_arc: Arc<Mutex<HashMap<SocketAddr, tokio::net::tcp::OwnedWriteHalf>>>,
) {
    assert!(!all_nodes.is_empty());

    // === initialization ===

    // parse identity keypair for current node
    let private_key_path = format!("{}/private{}.pk8", args.keys_dir, args.node_number);
    let mut identity_raw = std::fs::read(private_key_path).unwrap();
    let identity = Keypair::rsa_from_pkcs8(&mut identity_raw).unwrap();

    // our state consists of one file:
    // - `node_state.bin`: persistent state for this node
    // if we are clearing state, remove these two files first.
    let data_dir = args
        .data_dir
        .as_ref()
        .expect("`args.data_dir` should be `Some`");
    let _ = fs::create_dir_all(&data_dir);

    let node_state_path: PathBuf = [&data_dir, "node_state.bin"].iter().collect();
    if args.clear_data {
        // ignore errors
        let _ = fs::remove_file(&node_state_path);
    }

    // set up network
    let mut network: Network<Raft> =
        Network::initialize(identity, all_nodes, &all_nodes[0], true, node_state_path).unwrap();

    // load state, if present
    // TODO

    // Future that indicates when a heartbeat has not been received for some time
    let heartbeat_timeout = heartbeat::new_random(args.election_timeout);
    tokio::pin!(heartbeat_timeout);

    // Future that is resolved whenever a leader should send a heartbeat
    let should_send_heartbeat = heartbeat::new_random(args.heartbeat_interval);
    tokio::pin!(should_send_heartbeat);

    // future to trigger shutdown
    let mut shutdown = shutdown::Shutdown::new(shutdown_rx);

    // === event loop ===
    info!("node {} ready!", args.node_number);
    loop {
        select! {
            // server shutdown
            () = shutdown.recv() => { return }

            // heartbeat not received: send timeout
            () = &mut heartbeat_timeout => {
                network.timeout();

                // reset the timer so we don't spam messages
                heartbeat::reset(heartbeat_timeout.as_mut(), args.election_timeout);
            }

            // time to send a heartbeat
            () = &mut should_send_heartbeat => {
                network.send_heartbeat();

                // reset the timer for the next one
                heartbeat::reset(should_send_heartbeat.as_mut(), args.heartbeat_interval);
            }

            // receive a new client request
            Ok((req, addr)) = request_rx.recv() => {
                // check if leader
                let res = match network.is_leader() {
                    false => command::Response::NotLeader,
                    true => match req {
                        command::Request::Get { key } => {
                            let val = network.check_output(key.clone());
                            debug!("(logic): GET key <{key}>: found <{val:?}>");
                            command::Response::GetR {key, val}
                        },
                        command::Request::Put { key, val } => {
                            network.broadcast((key.clone(), val.clone()));
                            debug!("(logic): PUT key: <{key}> value: <{val:#?}>");
                            command::Response::PutR {key, val}
                        },
                    }
                };

                info!("(logic): responding to {addr} with {res:?}");
                // send response over the network
                let mut write_streams = write_streams_arc
                    .lock()
                    .expect("should be able to lock mutex");
                let write_stream = write_streams
                    .get_mut(&addr)
                    .expect("should be able to find write stream for this address");
                let res_str = command::pack_response(res);
                socket::write_str_to_socket(write_stream, res_str).await;
                std::mem::drop(write_streams);
            }

            // poll the network driver, to process new connections and events.
            res = network.poll() => {
                match res {
                    NetworkPollResult::ProtocolEvent => {
                        // reset the heartbeat timer
                        heartbeat::reset(heartbeat_timeout.as_mut(), args.election_timeout);
                    }
                    NetworkPollResult::OtherEvent => {}
                }
            }
        }
    }
}

async fn start_client_handler(
    mut tcp_stream: tokio::net::tcp::OwnedReadHalf,
    addr: SocketAddr,
    request_tx: broadcast::Sender<(command::Request, SocketAddr)>,
    shutdown_rx: broadcast::Receiver<()>,
) {
    // TODO: properly shut down when the tcp connection is closed
    let mut shutdown = shutdown::Shutdown::new(shutdown_rx);
    info!("({addr}): started");

    loop {
        // wait for tcp stream to be readable
        tcp_stream
            .readable()
            .await
            .expect("tcp stream should be readable");

        // drive event loop
        tokio::select! {
            // shutdown signal received
            () = shutdown.recv() => { return }

            // new client request
            Ok(msg_len) = tcp_stream.read_u32_le() => {
                // read message length
                let msg_len : usize = msg_len
                    .try_into()
                    .expect("should be able to convert a u32 to a usize");

                let msg_str = socket::read_bytes_from_socket_to_str(&mut tcp_stream, msg_len).await;
                info!("({addr}): received {msg_str}");
                let req = command::parse_request(msg_str).expect("message should be well-formed");

                // forward message to request channel
                request_tx
                    .send((req, addr))
                    .expect("should be able to forward request");
            }
        }
    }
}

/// a simple distributed key-value store (for strings), using the raft protocol
/// each instance runs a single node/replica, running across several threads:
/// - (server thread)  handles cli-input from the user and accepts incoming tcp connections.
///                    for an instance with node number n, the server runs on port (8000 + n).
///                    node number is 1-indexed.
/// - (consensus thread) handles communication within the network, i.e., it talks to other nodes
///   to reach consensus
/// - (client handler threads) handle communication with clients, i.e. it recieves requests and sends
///   responses from/to clients
#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
pub async fn main() {
    env_logger::init();

    // parse command-line args
    let mut args = Args::parse();

    let all_nodes: Vec<PeerId> = (1..=args.total_nodes)
        .map(|i| format!("{}/public{}.der", args.keys_dir, i))
        .map(|pk_path| parse_public_key(&pk_path))
        .collect();

    // initialize default data directory, if not provided
    // after this point, `args.data_dir` is always `Some`.
    let _ = args
        .data_dir
        .get_or_insert(format!("data/node_{}", args.node_number));

    // create shutdown channel
    let (shutdown_tx, shutdown_rx) = broadcast::channel(BCAST_CHANNEL_CAPACITY);

    // create command channels
    let (request_tx, request_rx) = broadcast::channel(BCAST_CHANNEL_CAPACITY);
    let write_streams_arc: Arc<Mutex<HashMap<SocketAddr, tokio::net::tcp::OwnedWriteHalf>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // spawn driver thread
    let write_streams_driver = write_streams_arc.clone();
    tokio::task::spawn(async move {
        // TODO: see if stdin handling is still necessary
        let mut stdin = io::BufReader::new(io::stdin()).lines();

        // set up tcp listener to handle client connections
        let host = "127.0.0.1";
        let port = 8000 + args.node_number;
        let addr = format!("{host}:{port}");
        let tcp_listener = TcpListener::bind(addr)
            .await
            .expect("should be able to create tcp listener");

        info!("driver thread ready!");

        loop {
            select! {
                // handle stdin (only for quitting)
                Ok(Some(line)) = stdin.next_line() => {
                    if line == "q" {
                        println!("quitting");
                        // handle shutdown
                        let _ = shutdown_tx.send(());
                        return;
                    } else {
                        println!("unrecognised: {line}");
                    }
                }

                // handle new tcp connection
                Ok((tcp_stream, addr)) = tcp_listener.accept() => {
                    println!("accepting new tcp connection at {addr}");

                    // add the write stream to the connections table
                    let (read_stream, write_stream) = tcp_stream.into_split();
                    let mut write_streams = write_streams_driver
                        .lock()
                        .expect("should be able to lock mutex");
                    write_streams.insert(addr, write_stream);
                    std::mem::drop(write_streams);

                    // set up other channels
                    let request_tx = request_tx.clone();
                    let shutdown_rx = shutdown_tx.subscribe();

                    tokio::task::spawn(async move {
                        start_client_handler(read_stream, addr, request_tx, shutdown_rx).await
                    });
                }

            }
        }
    });

    // run logic thread (in this thread)
    start_node(
        args.clone(),
        &all_nodes,
        shutdown_rx,
        request_rx,
        write_streams_arc,
    )
    .await
}
