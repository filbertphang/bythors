use super::entry::RaftEntry;

use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::marshal::array::lean_array_to_rust_vec;
use crate::marshal::core::{lean_dec_cond, lean_option_to_rust};
use crate::marshal::string::lean_string_to_rust;

use lean_sys::*;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct RaftPersistentState {
    current_term: usize,
    voted_for: Option<String>,
    log: Vec<RaftEntry>,
}

impl std::fmt::Display for RaftPersistentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <RaftPersistentState as std::fmt::Debug>::fmt(&self, f)
    }
}

impl RaftPersistentState {
    pub unsafe fn from_lean(persistent_state_lean: *mut lean_object, dec_refcount: bool) -> Self {
        // in lean, this struct is encoded as a 3-tuple
        let current_term_lean = lean_ctor_get(persistent_state_lean, 0);
        let rest = lean_ctor_get(persistent_state_lean, 1);
        let voted_for_lean = lean_ctor_get(rest, 0);
        let log_lean = lean_ctor_get(rest, 1);

        let current_term = lean_usize_of_nat(current_term_lean);
        let voted_for =
            lean_option_to_rust(voted_for_lean, |s| lean_string_to_rust(s, false), false);
        let log = lean_array_to_rust_vec(log_lean, |o| RaftEntry::from_lean(o, false), false);

        lean_dec_cond(persistent_state_lean, dec_refcount);

        Self {
            current_term,
            voted_for,
            log,
        }
    }

    pub fn to_disk(&self, write_path: &Path) {
        let bytes =
            bitcode::serialize(self).expect("should be able to serialize RaftPersistentState");
        let mut file_to_write =
            File::create(write_path).expect("should be able to open file in write-only mode");
        file_to_write
            .write_all(&bytes)
            .expect("should be able to write serialized RaftPersistentState to disk");
        file_to_write
            .flush()
            .expect("should be able to flush written data to file");
    }
}
