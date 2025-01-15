use crate::globals;
use crate::marshal::array::{index_lean_array, rust_vec_to_lean_array};
use crate::marshal::string::{lean_string_to_rust, rust_string_to_lean};
use crate::protocol::lean_extern;
use crate::protocol::message::Message;
use crate::protocol::packet::Packet;
use lean_sys::*;
use log::info;
use std::collections::HashSet;

pub use crate::protocol::lean_extern::initialize_ReliableBroadcastConcrete;

#[derive(Debug)]
pub struct Protocol {
    protocol: *mut lean_object,
    node_state: *mut lean_object,
    round: usize,
    // this field denotes which rounds are still pending consensus.
    // when a new round is started, the round number is added to the hash set.
    // when we reach consensus for that round, it is removed from the hash set.
    reached_consensus: HashSet<usize>,
    leader: String,
}

impl Protocol {
    pub unsafe fn create(node_list: Vec<String>, address: String, leader: String) -> Self {
        // initialize protocol
        let node_array_lean = rust_vec_to_lean_array(node_list, rust_string_to_lean);
        let protocol = lean_extern::create_protocol(node_array_lean);

        // initialize this node's state
        let node_address_lean = rust_string_to_lean(address);

        // exported lean functions take ownership of their parameters.
        // RC: increment reference count of the protocol object, so that we can continue
        // to use it after the function call.
        lean_inc(protocol);
        let node_state = lean_extern::init_node_state(protocol, node_address_lean);

        // initialize the global message hashtbl and consensus state
        globals::message_hashtbl::initialize();
        let reached_consensus = HashSet::new();

        // initialize round to 0
        let round = 0;

        Self {
            protocol,
            node_state,
            round,
            reached_consensus,
            leader,
        }
    }

    /// Deconstructs a Lean (new_state, packets_to_send) tuple into its Rust
    /// representation.
    /// This function TAKES OWNERSHIP of `state_and_packets`, and returns ownership of
    /// the new state and packet vector.
    unsafe fn deconstruct_state_and_packets(
        state_and_packets: *mut lean_object,
    ) -> (*mut lean_object, Vec<Packet>) {
        // deconstruct new protocol state
        assert!(lean_is_ctor(state_and_packets));
        assert!(lean_ctor_num_objs(state_and_packets) == 2);
        let new_state = lean_ctor_get(state_and_packets, 0);

        // deconstruct lean packets into rust
        // RC: `lean_ctor_get` does not seem to increment the ref count.
        // we do not have to free `packets_arr_lean` later.
        let packets_arr_lean = lean_ctor_get(state_and_packets, 1);
        let n_packets: usize = lean_array_size(packets_arr_lean);
        let mut packets_to_send = Vec::new();
        for i in 0..n_packets {
            // RC: indexing into the array increments the RC of the packet.
            let packet_lean = index_lean_array(packets_arr_lean, i);

            // unmarshall the packet into rust.
            // RC: since we incremeneted refcount when indexing, decrement it after we're done marshalling.
            let packet_rust = Packet::from_lean(packet_lean, true);
            packets_to_send.push(packet_rust);
        }

        // RC: incrementing refcount of `new_state`, since we return it from this function.
        lean_inc(new_state);

        // RC: decrement refcount of the result tuple, which should free packet array but NOT new state.
        lean_dec(state_and_packets);

        (new_state, packets_to_send)
    }

    /// Starts a new round of consensus with a given message.
    /// Sends the message to all other nodes.
    pub unsafe fn start_round(&mut self, address: String, message: String) -> Vec<Packet> {
        // add the current message to the global message table
        globals::message_hashtbl::insert(address, message);

        // send the InitialMessage
        // RC: increment refcount of `protocol`, since passing it into `send_message` gives it ownership,
        // and we need it to persist after the function call.
        // `node_state` need not be incremented, since it will be outdated after the message is sent,
        // and it is safe to free.
        lean_inc(self.protocol);
        let state_and_packets =
            lean_extern::send_message(self.protocol, self.node_state, self.round);

        let (new_state, packets_to_send) = Self::deconstruct_state_and_packets(state_and_packets);

        // update node state
        self.node_state = new_state;

        // increment round
        // TODO: this is maintained per-node for now, but eventually we may want some way
        // to broadcast the fact that we're starting a new round to all nodes.
        self.round += 1;

        packets_to_send
    }

    pub unsafe fn handle_packet(&mut self, packet: Packet) -> Vec<Packet> {
        // debug print
        info!("received packet:\n{}", packet);

        let src_lean = rust_string_to_lean(packet.src);
        let msg_lean = Message::to_lean(packet.msg);

        // RC: increment refcount of `protocol`, since passing it into `send_message` gives it ownership,
        // and we need it to persist after the function call.
        // `node_state`, `src_lean`, and `msg_lean` can be safely discarded after.
        lean_inc(self.protocol);
        let state_and_packets =
            lean_extern::handle_message(self.protocol, self.node_state, src_lean, msg_lean);

        let (new_state, packets_to_send) = Self::deconstruct_state_and_packets(state_and_packets);

        // update node state
        self.node_state = new_state;

        packets_to_send
    }

    // TODO: figure out `check_output` later.
    pub unsafe fn check_output(&mut self, round: usize) -> Option<String> {
        if self.reached_consensus.contains(&round) {
            // we have already achieved consensus for this round, no need to return the output
            return None;
        } else {
            let leader = rust_string_to_lean(self.leader.clone());

            lean_inc(self.node_state);
            let output_opt_lean = lean_extern::check_output(self.node_state, leader, round);

            // note: the runtime representation of lean4 options are:
            // - none: lean_box(0), which is a scalar
            // - some x: a constructor with 1 parameter, where that parameter is probably x
            match lean_is_scalar(output_opt_lean) {
                true => {
                    // consensus not yet reached for this round
                    None
                }
                false => {
                    // consensus reached, return output value
                    let output_lean = lean_ctor_get(output_opt_lean, 0);
                    let output = lean_string_to_rust(output_lean, false);
                    self.reached_consensus.insert(round);

                    Some(output)
                }
            }
        }
    }
}
