use super::Packet;

use lean_sys::*;

pub trait Protocol {
    type Message;

    unsafe fn initialize_lean(builtin: u8, world: lean_obj_arg) -> lean_obj_res;

    unsafe fn create(node_list: Vec<String>, address: String, leader: String) -> Self;
    unsafe fn start_round(
        &mut self,
        address: String, // the node's own address
        message: String,
    ) -> Vec<Packet<Self::Message>>;
    unsafe fn handle_packet(&mut self, packet: Packet<Self::Message>)
        -> Vec<Packet<Self::Message>>;
    unsafe fn check_output(&mut self, round: usize) -> Option<String>;
}
