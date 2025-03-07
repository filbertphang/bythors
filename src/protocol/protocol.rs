use super::Packet;

use lean_sys::*;

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
}
