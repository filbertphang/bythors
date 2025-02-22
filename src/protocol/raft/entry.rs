use super::lean_extern;

use crate::marshal::core::lean_dec_cond;
use crate::marshal::string::{lean_string_to_rust, rust_string_to_lean};

use lean_sys::*;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct RaftEntry {
    e_at: String,
    e_client: usize, // stored as nat
    e_id: usize,     // stored as nat
    e_index: usize,  // stored as nat
    e_term: usize,   // stored as nat
    e_input: String,
}

impl std::fmt::Display for RaftEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <RaftEntry as std::fmt::Debug>::fmt(&self, f)
    }
}

impl RaftEntry {
    unsafe fn from_lean(entry_lean: *mut lean_object, dec_refcount: bool) -> Self {
        let e_at_lean = lean_ctor_get(entry_lean, 0);
        let e_client_lean = lean_ctor_get(entry_lean, 1);
        let e_id_lean = lean_ctor_get(entry_lean, 2);
        let e_index_lean = lean_ctor_get(entry_lean, 3);
        let e_term_lean = lean_ctor_get(entry_lean, 4);
        let e_input_lean = lean_ctor_get(entry_lean, 5);

        let e_at = lean_string_to_rust(e_at_lean, false);
        let e_input = lean_string_to_rust(e_input_lean, false);

        let e_client = lean_usize_of_nat(e_client_lean);
        let e_id = lean_usize_of_nat(e_id_lean);
        let e_index = lean_usize_of_nat(e_index_lean);
        let e_term = lean_usize_of_nat(e_term_lean);

        // conditionally free the lean entry.
        lean_dec_cond(entry_lean, dec_refcount);

        Self {
            e_at,
            e_client,
            e_id,
            e_index,
            e_term,
            e_input,
        }
    }

    // Takes ownership of the Rust Message.
    unsafe fn to_lean(self) -> *mut lean_object {
        let e_at = rust_string_to_lean(self.e_at);
        let e_input = rust_string_to_lean(self.e_input);

        let e_client = lean_usize_to_nat(self.e_client);
        let e_id = lean_usize_to_nat(self.e_id);
        let e_index = lean_usize_to_nat(self.e_index);
        let e_term = lean_usize_to_nat(self.e_term);

        lean_extern::create_entry(e_at, e_client, e_id, e_index, e_term, e_input)
    }
}
