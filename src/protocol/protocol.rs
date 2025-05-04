use super::{raft::RaftPersistentState, Packet};

use lean_sys::*;

/// This protocol trait is outdated.
/// This trait was originally supposed to provide a standardized interface for a protocol,
/// back when we wanted to plug-and-play different protocols into the same application.
/// As the project evolved, this functionality was no longer in the scope of this project.
pub trait Protocol {
    type Message;

    unsafe fn initialize_lean(builtin: u8, world: lean_obj_arg) -> lean_obj_res;

    unsafe fn create(node_list: Vec<String>, address: String, leader: String) -> Self;
    unsafe fn start(&mut self) -> Vec<Packet<Self::Message>> {
        Vec::new()
    }

    unsafe fn start_round(
        &mut self,
        address: String, // the node's own address
        message: (String, String),
    ) -> Vec<Packet<Self::Message>>;
    unsafe fn handle_packet(&mut self, packet: Packet<Self::Message>)
        -> Vec<Packet<Self::Message>>;
    unsafe fn check_output(&mut self, key: String) -> Option<String>;

    unsafe fn handle_timeout(&mut self) -> Vec<Packet<Self::Message>>;
    unsafe fn send_heartbeat(&mut self) -> Vec<Packet<Self::Message>>;
    unsafe fn is_leader(&mut self) -> bool;
    unsafe fn get_persistent_state(&self) -> RaftPersistentState;
    unsafe fn load_persistent_state(&mut self, persistent_state: RaftPersistentState);
}
