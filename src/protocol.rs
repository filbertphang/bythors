mod message;
mod packet;
mod protocol;

// module re-exports
pub use message::Message;
pub use packet::Packet;
pub use protocol::Protocol;

// implemented protocols
pub mod reliable_broadcast {
    mod lean_extern;
    mod message;
    mod protocol;

    pub use message::RBMessage;
    pub use protocol::ReliableBroadcast;
}

pub mod raft {
    mod entry;
    mod lean_extern;
    mod message;
    mod protocol;

    pub use message::RaftMessage;
    pub use protocol::Raft;
}
