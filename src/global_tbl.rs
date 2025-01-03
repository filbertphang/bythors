use crate::marshal::string::{lean_string_to_rust, rust_string_to_lean};
use lean_sys::lean_object;
use once_cell::sync::OnceCell;
use std::{collections::HashMap, sync::Mutex};

// TODO: investigate an alternative implementation for the global message hashtable.
// Global state can be stored in either the IO Monad or some form of State monad (StateM) in Lean.
// The benefit of this is that state lives exactly in one place (NodeState) as opposed to being split across
// the NodeState and the global state on the Rust side, at the cost of additional hassle of working with monads.

// maps from Address (String) -> Message (String)
pub static GLOBAL_MESSAGE_HASHTBL: OnceCell<Mutex<HashMap<String, String>>> = OnceCell::new();

pub fn initialize() {
    GLOBAL_MESSAGE_HASHTBL
        .set(Mutex::new(HashMap::new()))
        .expect("should be able to initialize global message table");
}

#[no_mangle]
pub unsafe extern "C" fn get_node_value(node_address: *mut lean_object) -> *mut lean_object {
    let ht = GLOBAL_MESSAGE_HASHTBL
        .get()
        .expect("global message hashtbl should be initialized")
        .lock()
        .unwrap();

    let node_address_rust = lean_string_to_rust(node_address, true);

    let message_rust = ht
        .get(&node_address_rust)
        .expect("node should always have a message")
        .clone();

    rust_string_to_lean(message_rust)
}
