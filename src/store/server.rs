use std::net::SocketAddr;

use crate::network::{Network, NetworkPollResult};
use crate::protocol::raft::Raft;
use crate::store::{command, heartbeat, shutdown};

use clap::Parser;
use libp2p::identity::{rsa, Keypair, PeerId, PublicKey};
use log::info;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::{io, select};

const BCAST_CHANNEL_CAPACITY: usize = 32;

#[derive(Parser, Clone)]
struct Args {
    node_number: usize,
    total_nodes: usize,

    #[arg(default_value_t = 10000)]
    election_timeout: u64,

    #[arg(default_value_t = 2500)]
    heartbeat_interval: u64,

    // intended to be run from the crate base directory
    #[arg(default_value_t = String::from("keys"))]
    base_dir: String,
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
    response_tx: broadcast::Sender<(command::Response, SocketAddr)>,
) {
    assert!(!all_nodes.is_empty());

    // === initialization ===

    // parse identity keypair for current node
    let private_key_path = format!("{}/private{}.pk8", args.base_dir, args.node_number);
    let mut identity_raw = std::fs::read(private_key_path).unwrap();
    let identity = Keypair::rsa_from_pkcs8(&mut identity_raw).unwrap();

    // set up  network
    let mut network: Network<Raft> =
        Network::initialize(identity, all_nodes, &all_nodes[0], true).unwrap();

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
                match req {
                    command::Request::Get { key } => {
                        let val = network.check_output(key.clone());
                        println!("GET key {key}: found {val:?}");
                        let res = command::Response::GetR {key, val};
                        response_tx.send((res, addr)).expect("should be able to send response");
                    },
                    command::Request::Put { key, val } => {
                        network.broadcast((key.clone(), val.clone()));
                        println!("PUT key: {key} value: {val:#?}");
                    },
                }
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
    mut tcp_stream: tokio::net::TcpStream,
    addr: SocketAddr,
    request_tx: broadcast::Sender<(command::Request, SocketAddr)>,
    mut response_rx: broadcast::Receiver<(command::Response, SocketAddr)>,
    shutdown_rx: broadcast::Receiver<()>,
) {
    let mut shutdown = shutdown::Shutdown::new(shutdown_rx);
    info!("({addr}): started");

    loop {
        // drive event loop
        tokio::select! {
            // shutdown signal received
            () = shutdown.recv() => { return }

            // new response to forward to client
            Ok((res, res_addr)) = response_rx.recv() => {
                // this message is intended for this client handler
                if res_addr == addr {
                    // construct response message
                    let res_bytes = command::pack_response(res);
                    let res_len: u32 = res_bytes.len().try_into().expect("response message length should fit into a u32");

                    // write response to tcp stream
                    tcp_stream.writable().await.expect("tcp stream should be writable");
                    tcp_stream.write_u32_le(res_len).await.expect("should be able to write msg length to tcp stream");
                    tcp_stream.write(&res_bytes).await.expect("should be able to write message to tcp stream");
                }
            }

            // new client request
            Ok(()) = tcp_stream.readable() => {
                // read message length
                let msg_len : usize = tcp_stream
                    .read_u32_le()
                    .await
                    .expect("should be able to read request len")
                    .try_into()
                    .expect("should be able to convert a u32 to a usize");

                // read actual message into buf
                let mut buf = vec![0u8; msg_len];
                tcp_stream
                    .read_exact(&mut buf)
                    .await
                    .expect("should be able to read msg from tcp stream");

                // parse message into a Request
                let msg_str = String::from_utf8(buf).expect("should be able to convert msg into string");
                println!("({addr}): received {msg_str}");
                let req = command::parse_request(msg_str).expect("message should be well-formed");

                // send message to request channel
                request_tx
                    .send((req, addr))
                    .expect("should be able to forward request");
            }
        }
    }
}

/// a simple distributed key-value store (for strings), using the raft protocol
/// each instance runs a single node/replica, running across several threads:
/// - (driver thread)  handles cli-input from the user and accepts incoming tcp connections
/// - (consensus thread) handles communication within the network, i.e., it talks to other nodes
///   to reach consensus
/// - (client handler threads) handle communication with clients, i.e. it recieves requests and sends
///   responses from/to clients
#[tokio::main]
pub async fn main() {
    env_logger::init();

    // parse command-line args
    let args = Args::parse();

    // use key 1 as master node
    // use keys 2 - 4 as replicas
    let all_nodes: Vec<PeerId> = (1..=args.total_nodes)
        .map(|i| format!("{}/public{}.der", args.base_dir, i))
        .map(|pk_path| parse_public_key(&pk_path))
        .collect();

    // create shutdown channel
    let (shutdown_tx, shutdown_rx) = broadcast::channel(BCAST_CHANNEL_CAPACITY);

    // create command channels
    let (request_tx, request_rx) = broadcast::channel(BCAST_CHANNEL_CAPACITY);
    let (response_tx, _) = broadcast::channel(BCAST_CHANNEL_CAPACITY);

    // spawn driver thread
    let response_tx_driver = response_tx.clone();
    tokio::task::spawn(async move {
        // spawn libp2p network (for consensus)
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
                    let request_tx = request_tx.clone();
                    let response_rx = response_tx_driver.subscribe();
                    let shutdown_rx = shutdown_tx.subscribe();
                    tokio::task::spawn(async move {
                        start_client_handler(tcp_stream, addr, request_tx, response_rx, shutdown_rx).await
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
        response_tx,
    )
    .await
}
