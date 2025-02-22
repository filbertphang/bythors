use super::entry::RaftEntry;
use super::lean_extern;

use crate::marshal::array::{lean_array_to_rust_vec, rust_vec_to_lean_array};
use crate::marshal::core::{bool_to_u8, lean_dec_cond, VOID_PTR_SIZE};
use crate::marshal::string::{lean_string_to_rust, rust_string_to_lean};
use crate::protocol::Message;

use lean_sys::*;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum RaftMessage {
    RequestVote {
        term: usize, // stored as nat
        candidate_id: String,
        last_log_index: usize, // stored as nat
        last_log_term: usize,  // stored as nat
    },
    RequestVoteReply {
        term: usize, // stored as nat
        vote_granted: bool,
    },
    AppendEntries {
        term: usize, // stored as nat
        leader_id: String,
        prev_log_index: usize, // stored as nat
        prev_log_term: usize,  // stored as nat
        entries: Vec<RaftEntry>,
        leader_commit: usize, // stored as nat
    },
    AppendEntriesReply {
        term: usize, // stored as nat
        entries: Vec<RaftEntry>,
        success: bool,
    },
}

impl std::fmt::Display for RaftMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <RaftMessage as std::fmt::Debug>::fmt(&self, f)
    }
}

impl Message for RaftMessage {
    fn get_round(&self) -> Option<usize> {
        // for Raft, we treat a "round" as reaching consensus on a client request.
        match self {
            Self::RequestVote { .. } | Self::RequestVoteReply { .. } => None,
            Self::AppendEntries { entries, .. } | Self::AppendEntriesReply { entries, .. } => {
                let last_entry = entries.last();
                last_entry.map(|e| e.e_client)
            }
        }
    }

    unsafe fn from_lean(msg_lean: *mut lean_object, dec_refcount: bool) -> Self {
        let tag = lean_ptr_tag(msg_lean);
        let field = |i| lean_ctor_get(msg_lean, i);

        let term = lean_usize_of_nat(field(0));

        // construct Rust message
        let msg = match tag {
            0 => {
                let candidate_id = lean_string_to_rust(field(1), false);
                let last_log_index = lean_usize_of_nat(field(2));
                let last_log_term = lean_usize_of_nat(field(3));

                Self::RequestVote {
                    term,
                    candidate_id,
                    last_log_index,
                    last_log_term,
                }
            }
            1 => {
                let offset: std::ffi::c_uint = VOID_PTR_SIZE.try_into().unwrap();
                let vote_granted = lean_ctor_get_uint8(msg_lean, offset) != 0;

                Self::RequestVoteReply { term, vote_granted }
            }
            2 => {
                let leader_id = lean_string_to_rust(field(1), false);
                let prev_log_index = lean_usize_of_nat(field(2));
                let prev_log_term = lean_usize_of_nat(field(3));
                let entries = lean_array_to_rust_vec(
                    field(4),
                    |elem| RaftEntry::from_lean(elem, false),
                    false,
                );
                let leader_commit = lean_usize_of_nat(field(5));

                Self::AppendEntries {
                    term,
                    leader_id,
                    prev_log_index,
                    prev_log_term,
                    entries,
                    leader_commit,
                }
            }
            3 => {
                let entries = lean_array_to_rust_vec(
                    field(2),
                    |elem| RaftEntry::from_lean(elem, false),
                    false,
                );
                let offset: std::ffi::c_uint = (2 * VOID_PTR_SIZE).try_into().unwrap();
                let success = lean_ctor_get_uint8(msg_lean, offset) != 0;

                Self::AppendEntriesReply {
                    term,
                    entries,
                    success,
                }
            }
            _ => panic!("unexpected tag"),
        };

        // conditionally free the lean message.
        lean_dec_cond(msg_lean, dec_refcount);

        msg
    }

    // Takes ownership of the Rust Message.
    unsafe fn to_lean(self) -> *mut lean_object {
        match self {
            Self::RequestVote {
                term,
                candidate_id,
                last_log_index,
                last_log_term,
            } => {
                let term_lean = lean_usize_to_nat(term);
                let candidate_id_lean = rust_string_to_lean(candidate_id);
                let last_log_index_lean = lean_usize_to_nat(last_log_index);
                let last_log_term_lean = lean_usize_to_nat(last_log_term);

                lean_extern::create_requestvote(
                    term_lean,
                    candidate_id_lean,
                    last_log_index_lean,
                    last_log_term_lean,
                )
            }
            Self::RequestVoteReply { term, vote_granted } => {
                let term_lean = lean_usize_to_nat(term);
                let vote_granted_lean = bool_to_u8(vote_granted);

                lean_extern::create_requestvotereply(term_lean, vote_granted_lean)
            }
            Self::AppendEntries {
                term,
                leader_id,
                prev_log_index,
                prev_log_term,
                entries,
                leader_commit,
            } => {
                let term_lean = lean_usize_to_nat(term);
                let leader_id_lean = rust_string_to_lean(leader_id);
                let prev_log_index_lean = lean_usize_to_nat(prev_log_index);
                let prev_log_term_lean = lean_usize_to_nat(prev_log_term);
                let entries_lean = rust_vec_to_lean_array(entries, RaftEntry::to_lean);
                let leader_commit_lean = lean_usize_to_nat(leader_commit);

                lean_extern::create_appendentries(
                    term_lean,
                    leader_id_lean,
                    prev_log_index_lean,
                    prev_log_term_lean,
                    entries_lean,
                    leader_commit_lean,
                )
            }
            Self::AppendEntriesReply {
                term,
                entries,
                success,
            } => {
                let term_lean = lean_usize_to_nat(term);
                let entries_lean = rust_vec_to_lean_array(entries, RaftEntry::to_lean);
                let success_lean = bool_to_u8(success);

                lean_extern::create_appendentriesreply(term_lean, entries_lean, success_lean)
            }
        }
    }
}
