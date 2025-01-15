use std::fmt::Debug;

use crate::marshal::core::{lean_dec_cond, VOID_PTR_SIZE};
use crate::marshal::string::lean_string_to_rust;
use crate::protocol::message::Message;
use lean_sys::*;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Packet {
    pub src: String,
    pub dst: String,
    pub msg: Message,
    pub consumed: bool,
}

impl std::fmt::Display for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Packet as std::fmt::Debug>::fmt(&self, f)
    }
}

impl Packet {
    pub fn get_round(&self) -> usize {
        self.msg.get_round()
    }

    // TODO (old): check if the convention should be `from_lean` or `of_lean`.
    /// Converts a Lean packet to its Rust representation.
    pub unsafe fn from_lean(packet_lean: *mut lean_object, dec_refcount: bool) -> Self {
        let src_lean = lean_ctor_get(packet_lean, 0);
        let dst_lean = lean_ctor_get(packet_lean, 1);
        let msg_lean = lean_ctor_get(packet_lean, 2);

        let consumed_lean_offset: std::ffi::c_uint = (3 * VOID_PTR_SIZE).try_into().unwrap();
        let consumed_lean = lean_ctor_get_uint8(packet_lean, consumed_lean_offset);

        // we should not have to free any of the packet components, since they should
        // automatically be freed once the packet itself is freed.
        // it should suffice to decrement just the refcount of the packet.
        let src = lean_string_to_rust(src_lean, false);
        let dst = lean_string_to_rust(dst_lean, false);
        let msg = Message::from_lean(msg_lean, false);

        // no proper way to cast u8 to bool, so we do this instead
        let consumed: bool = consumed_lean != 0;

        // free the lean packet, conditionally
        lean_dec_cond(packet_lean, dec_refcount);

        Packet {
            src,
            dst,
            msg,
            consumed,
        }
    }
}
