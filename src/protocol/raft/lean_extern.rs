use lean_sys::{lean_obj_arg, lean_obj_res};

#[link(name = "RaftConcreteFat", kind = "static")]
extern "C" {
    pub fn initialize_RaftConcrete(builtin: u8, world: lean_obj_arg) -> lean_obj_res;

    pub fn raft_init(
        me: lean_obj_arg,    // ConcreteAddress (String)
        nodes: lean_obj_arg, // List ConcreteAddress
    ) -> lean_obj_res; // ConcreteRaftData

    pub fn raft_handle_input(
        state: lean_obj_arg,     // ConcreteRaftData
        client_id: lean_obj_arg, // ClientId
        value: lean_obj_arg,     // ConcreteValue
    ) -> lean_obj_res; // (ConcreteRaftData, List ConcreteRaftInput, List ConcreteRaftPacket)

    pub fn raft_handle_timeout(state: lean_obj_arg, // ConcreteRaftData
    ) -> lean_obj_res; // (ConcreteRaftData, List ConcreteRaftInput, List ConcreteRaftPacket)

    pub fn raft_handle_message(
        state: lean_obj_arg, // ConcreteRaftData
        src: lean_obj_arg,   // ConcreteAddress
        msg: lean_obj_arg,   // ConcreteRaftMessage
    ) -> lean_obj_res; // (ConcreteRaftData, List ConcreteRaftInput, List ConcreteRaftPacket)

    pub fn raft_check_output(
        state: lean_obj_arg, // ConcreteRaftData
        index: lean_obj_arg, // ClientId
    ) -> lean_obj_res; // Option ConcreteValue

    pub fn raft_create_entry(
        e_at: lean_obj_arg,
        e_client: lean_obj_arg,
        e_id: lean_obj_arg,
        e_index: lean_obj_arg,
        e_term: lean_obj_arg,
        e_input: lean_obj_arg,
    ) -> lean_obj_res;

    pub fn raft_create_requestvote(
        term: lean_obj_arg,
        candidate_id: lean_obj_arg,
        last_log_index: lean_obj_arg,
        last_log_term: lean_obj_arg,
    ) -> lean_obj_res;

    pub fn raft_create_requestvotereply(term: lean_obj_arg, vote_granted: u8) -> lean_obj_res;

    pub fn raft_create_appendentries(
        term: lean_obj_arg,
        leader_id: lean_obj_arg,
        prev_log_index: lean_obj_arg,
        prev_log_term: lean_obj_arg,
        entries: lean_obj_arg,
        leader_commit: lean_obj_arg,
    ) -> lean_obj_res;

    pub fn raft_create_appendentriesreply(
        term: lean_obj_arg,
        entries: lean_obj_arg,
        success: u8,
    ) -> lean_obj_res;

    pub fn raft_convert_entry_list_to_arr(entries: lean_obj_arg) -> lean_obj_res;
}
