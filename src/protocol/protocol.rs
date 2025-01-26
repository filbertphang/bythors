use lean_sys::*;

pub trait Protocol {
    type Packet;

    unsafe fn initialize_lean(builtin: u8, world: lean_obj_arg) -> lean_obj_res;

    unsafe fn create(node_list: Vec<String>, address: String, leader: String) -> Self;
    unsafe fn start_round(&mut self, address: String, message: String) -> Vec<Self::Packet>;
    unsafe fn handle_packet(&mut self, packet: Self::Packet) -> Vec<Self::Packet>;
    unsafe fn check_output(&mut self, round: usize) -> Option<String>;
}
