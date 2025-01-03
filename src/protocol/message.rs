use crate::marshal::core::lean_dec_cond;
use crate::marshal::string::{lean_string_to_rust, rust_string_to_lean};
use crate::protocol::lean_extern::create_message;
use lean_sys::*;

// note: even though the lean representation looks identical to this enum type,
// the memory representation is different.
// since `USize` fields are ordered AFTER `lean_object` fields, each constructor would look like:
// `| EchoMsg { originator: String, v: String, r: usize }`, and we have to deconstruct it in that order.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum Message {
    InitialMsg {
        r: usize,
        v: String,
    },
    EchoMsg {
        originator: String,
        r: usize,
        v: String,
    },
    VoteMsg {
        originator: String,
        r: usize,
        v: String,
    },
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::InitialMsg { r, v } => write!(f, "InitialMsg @ round {}: {}", r, v),
            Message::EchoMsg { originator, r, v } => {
                write!(f, "EchoMsg from {} @ round {}: {}", originator, r, v)
            }
            Message::VoteMsg { originator, r, v } => {
                write!(f, "VoteMsg from {} @ round {}: {}", originator, r, v)
            }
        }
    }
}

impl Message {
    pub fn get_round(&self) -> usize {
        match &self {
            Self::InitialMsg { r, .. } | Self::EchoMsg { r, .. } | Self::VoteMsg { r, .. } => *r,
        }
    }

    pub unsafe fn from_lean(msg_lean: *mut lean_object, dec_refcount: bool) -> Self {
        let tag = lean_ptr_tag(msg_lean);
        let mut current_field_id = 0;

        // only EchoMsg and VoteMsg have the originator fields.
        let mut originator: String = String::new();
        if tag == 1 || tag == 2 {
            originator = lean_string_to_rust(lean_ctor_get(msg_lean, current_field_id), false);
            current_field_id += 1;
        }

        // TODO: see if we can replace ctor_get + unbox with `lean_ctor_get_usize`.
        let r_lean = lean_ctor_get(msg_lean, current_field_id);
        let r: usize = lean_unbox_usize(r_lean);
        current_field_id += 1;

        // TODO (old): there is some dangling pointer issue here.
        // something about the way the packet list gets returned.
        // basically, it seems like `v` is freed after the first packet or something, so
        // we cannot use it again for the second packet?
        let v_lean = lean_ctor_get(msg_lean, current_field_id);

        // TODO (old): temporarily convert the lean string to rust as borrowed, since it's shared.
        // handle memory leaks later.
        let v = lean_string_to_rust(v_lean, false);

        // conditionally free the lean message.
        lean_dec_cond(msg_lean, dec_refcount);

        // construct Rust message
        match tag {
            0 => Message::InitialMsg { r, v },
            1 => Message::EchoMsg { originator, r, v },
            2 => Message::VoteMsg { originator, r, v },
            _ => panic!("unexpected tag"),
        }
    }

    // Takes ownership of the Rust Message.
    pub unsafe fn to_lean(self) -> *mut lean_object {
        let tag: usize;
        let originator_r: String;
        let r_r: usize;
        let v_r: String;

        match self {
            Self::InitialMsg { r, v } => {
                tag = 0;
                // hacky way to include this.
                // will not be used on the Lean side.
                originator_r = String::new();
                r_r = r;
                v_r = v;
            }
            Self::EchoMsg { originator, r, v } => {
                tag = 1;
                originator_r = originator;
                r_r = r;
                v_r = v;
            }
            Self::VoteMsg { originator, r, v } => {
                tag = 2;
                originator_r = originator;
                r_r = r;
                v_r = v;
            }
        };

        create_message(
            tag,
            rust_string_to_lean(originator_r),
            r_r,
            rust_string_to_lean(v_r),
        )
    }
}
