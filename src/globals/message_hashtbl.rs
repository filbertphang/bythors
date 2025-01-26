use crate::marshal::string::{lean_string_to_rust, rust_string_to_lean};
use lean_sys::lean_object;
use once_cell::sync::OnceCell;
use std::{collections::HashMap, sync::Mutex};

// TODO: investigate an alternative implementation for the global message hashtable.
// Global state can be stored in either the IO Monad or some form of State monad (StateM) in Lean.
// The benefit of this is that state lives exactly in one place (NodeState) as opposed to being split across
// the NodeState and the global state on the Rust side, at the cost of additional hassle of working with monads.

// TODO: should global message hashtbl be an implementation detail for reliable broadcast?

// maps from Address (String) -> Message (String)
static GLOBAL_MESSAGE_HASHTBL: OnceCell<Mutex<HashMap<String, String>>> = OnceCell::new();

pub fn initialize() {
    GLOBAL_MESSAGE_HASHTBL
        .set(Mutex::new(HashMap::new()))
        .expect("should be able to initialize global message table");
}

pub fn insert(address: String, message: String) {
    let mut ht = GLOBAL_MESSAGE_HASHTBL
        .get()
        .expect("expected global message db to be initialized")
        .lock()
        .unwrap();

    ht.insert(address, message);

    // note: no need to manually release the mutex on `ht`, since it will
    // go out of scope once the function returns.
}

pub fn get(address: String) -> String {
    let ht = GLOBAL_MESSAGE_HASHTBL
        .get()
        .expect("expected global message db to be initialized")
        .lock()
        .unwrap();

    ht.get(&address)
        .expect("expected an entry for the given address")
        .clone()
}

#[no_mangle]
pub unsafe extern "C" fn get_node_value(node_address: *mut lean_object) -> *mut lean_object {
    let node_address_rust = lean_string_to_rust(node_address, true);
    let message_rust = get(node_address_rust);
    rust_string_to_lean(message_rust)
}
