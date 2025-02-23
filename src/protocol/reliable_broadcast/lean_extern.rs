use lean_sys::{lean_obj_arg, lean_obj_res};

// note: we link with `ReliableBroadcastConcreteFat`, not `ReliableBroadcastConcrete`.
// this is because `ReliableBroadcastConcrete.lean` has several additional dependencies that we need to link with,
// so we export it as a "Fat" static library.
// see `lib/lakefile.lean` for more info.

// TODO: how do we allow the user to use their own lean protocols?
// i think we might have to change this to link to a dynamic library, since the user protocol
// may not be available at compile-time for this library (?)
// we then require the user to:
// 1) compile their lean protocol to a fat static library
// 2) link to their fat static library
// 3) re-export their fat static library as a dynamic library
// not too sure how this works too, will need to investigate more in a sandbox.
#[link(name = "ReliableBroadcastConcreteFat", kind = "static")]
extern "C" {
    pub fn initialize_ReliableBroadcastConcrete(builtin: u8, world: lean_obj_arg) -> lean_obj_res;

    pub fn rb_create_protocol(node_arr: lean_obj_arg) -> lean_obj_res;

    pub fn rb_create_message(
        tag: usize,
        originator: lean_obj_arg,
        r: usize,
        v: lean_obj_arg,
    ) -> lean_obj_res;

    pub fn rb_init_node_state(p: lean_obj_arg, node_address: lean_obj_arg) -> lean_obj_res;

    pub fn rb_send_message(p: lean_obj_arg, node_state: lean_obj_arg, round: usize)
        -> lean_obj_res;

    pub fn rb_handle_message(
        p: lean_obj_arg,
        node_state: lean_obj_arg,
        src: lean_obj_arg,
        msg: lean_obj_arg,
    ) -> lean_obj_res;

    pub fn rb_check_output(
        node_state: lean_obj_arg,
        leader: lean_obj_arg,
        round: usize,
    ) -> lean_obj_res;
}
