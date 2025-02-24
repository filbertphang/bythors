use super::lean_extern;
use super::message::RaftMessage;

// TODO 23/2/25 0003H impl this! heehee

use crate::globals;
use crate::marshal::array::{index_lean_array, rust_vec_to_lean_array};
use crate::marshal::core::lean_option_to_rust;
use crate::marshal::string::{lean_string_to_rust, rust_string_to_lean};
use crate::protocol::{Message, Packet, Protocol};

use lean_sys::*;
use log::info;
use std::collections::HashSet;

#[derive(Debug)]
pub struct Raft {
    node_state: *mut lean_object,
    round: usize, // same as ClientId
}

type RaftPacket = Packet<RaftMessage>;

impl Protocol for Raft {
    type Message = RaftMessage;

    unsafe fn initialize_lean(builtin: u8, world: lean_obj_arg) -> lean_obj_res {
        lean_extern::initialize_RaftConcrete(builtin, world)
    }

    unsafe fn create(node_list: Vec<String>, address: String, _leader: String) -> Self {
        // leader is ignored here, because Raft elections will automatically
        // result in a suitable leader to be chosen

        // initialize this node's state
        let node_array_lean = rust_vec_to_lean_array(node_list, rust_string_to_lean);
        let node_address_lean = rust_string_to_lean(address);

        // exported lean functions take ownership of their parameters.
        // RC: increment reference count of the protocol object, so that we can continue
        // to use it after the function call.
        let node_state = lean_extern::raft_init(node_address_lean, node_array_lean);

        // initialize round to 0
        let round = 0;

        Self { node_state, round }
    }

    /// Starts a new round of consensus with a given message.
    /// Sends the message to all other nodes.
    unsafe fn start_round(&mut self, _address: String, message: String) -> Vec<RaftPacket> {
        // (own address is not needed, since it's already stored in the node state)

        let round_lean = lean_usize_to_nat(self.round);
        let value_lean = rust_string_to_lean(message);
        let state_and_packets =
            lean_extern::raft_handle_input(self.node_state, round_lean, value_lean);

        let (new_state, packets_to_send) = deconstruct_state_and_packets(state_and_packets);

        // update node state
        self.node_state = new_state;

        // increment round
        // TODO: this is maintained per-node for now, but eventually we may want some way
        // to broadcast the fact that we're starting a new round to all nodes.
        self.round += 1;

        packets_to_send
    }

    /// Deconstructs a Lean (new_state, packets_to_send) tuple into its Rust
    /// representation.
    /// This function TAKES OWNERSHIP of `state_and_packets`, and returns ownership of
    /// the new state and packet vector.
    unsafe fn handle_packet(&mut self, packet: RaftPacket) -> Vec<RaftPacket> {
        // debug print
        info!("received packet:\n{}", packet);

        let src_lean = rust_string_to_lean(packet.src);
        let msg_lean = packet.msg.to_lean();

        // RC: increment refcount of `protocol`, since passing it into `send_message` gives it ownership,
        // and we need it to persist after the function call.
        // `node_state`, `src_lean`, and `msg_lean` can be safely discarded after.
        let state_and_packets =
            lean_extern::raft_handle_message(self.node_state, src_lean, msg_lean);

        let (new_state, packets_to_send) = deconstruct_state_and_packets(state_and_packets);

        // update node state
        self.node_state = new_state;

        packets_to_send
    }

    unsafe fn check_output(&mut self, round: usize) -> Option<String> {
        let round_lean = lean_usize_to_nat(round);
        let output_opt_lean = lean_extern::raft_check_output(self.node_state, round_lean);
        let output = lean_option_to_rust(output_opt_lean, |x| lean_string_to_rust(x, false), true);
        output
    }
}

unsafe fn deconstruct_state_and_packets(
    state_and_results_and_packets: *mut lean_object,
) -> (*mut lean_object, Vec<RaftPacket>) {
    // deconstruct new protocol state
    assert!(lean_is_ctor(state_and_results_and_packets));
    assert!(lean_ctor_num_objs(state_and_results_and_packets) == 2);
    let new_state = lean_ctor_get(state_and_results_and_packets, 0);

    let results_and_packets = lean_ctor_get(state_and_results_and_packets, 1);
    assert!(lean_ctor_num_objs(results_and_packets) == 2);
    // ignore the `List RaftOutput` at ctor field 1 (for now)

    // deconstruct lean packets into rust
    // RC: `lean_ctor_get` does not seem to increment the ref count.
    // we do not have to free `packets_arr_lean` later.
    let packets_arr_lean = lean_ctor_get(results_and_packets, 1);
    let n_packets: usize = lean_array_size(packets_arr_lean);
    let mut packets_to_send = Vec::new();
    for i in 0..n_packets {
        // RC: indexing into the array increments the RC of the packet.
        let packet_lean = index_lean_array(packets_arr_lean, i);

        // unmarshall the packet into rust.
        // RC: since we incremeneted refcount when indexing, decrement it after we're done marshalling.
        let packet_rust = RaftPacket::from_lean(packet_lean, true);
        packets_to_send.push(packet_rust);
    }

    // RC: incrementing refcount of `new_state`, since we return it from this function.
    lean_inc(new_state);

    // RC: decrement refcount of the result tuple, which should free packet array but NOT new state.
    lean_dec(results_and_packets);

    (new_state, packets_to_send)
}
