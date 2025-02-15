import LeanSts.State
import LeanSts.BFT.Network
import Raft

-- lean-rust interfacing
-- ===
-- we construct concrete versions of the protocol in lean so that we do not need to pass type parameters
-- between Lean and Rust.

-- concrete address, and value parameters
@[reducible] def ConcreteAddress := String
@[reducible] def ConcreteValue := String

-- temporarily instantiate state machine as the identity machine
@[reducible] def ConcreteStateMachineData := Unit
def smdInit := ()

-- concrete instantiations of Raft types
abbrev ConcreteRaftEntry := @Entry ConcreteAddress ConcreteValue
abbrev ConcreteRaftMessage := @Message ConcreteAddress ConcreteValue
abbrev ConcreteRaftData := @Data ConcreteAddress ConcreteValue ConcreteStateMachineData
abbrev ConcreteRaftInput := @Input ConcreteValue
abbrev ConcreteRaftOutput := @Output ConcreteValue
abbrev ConcreteRaftPacket := @Packet ConcreteAddress ConcreteRaftMessage

-- callback function for when consensus is reached on a log entry
@[extern "run_state_machine"]
opaque run_state_machine : ConcreteValue → ConcreteStateMachineData → ConcreteValue × ConcreteStateMachineData

@[export raft_init]
def raft_init (me : ConcreteAddress) (nodes : List ConcreteAddress)
  : ConcreteRaftData :=
  init_handlers smdInit me nodes

@[export raft_handle_message]
def raft_handle_message
  (state : ConcreteRaftData)
  (src : ConcreteAddress)
  (msg : ConcreteRaftMessage)
  : (ConcreteRaftData × List ConcreteRaftOutput × List ConcreteRaftPacket) :=
  RaftNetHandler run_state_machine src msg state

@[export raft_handle_input]
def raft_handle_input
  (state : ConcreteRaftData)
  (input : ConcreteRaftInput)
  : (ConcreteRaftData × List ConcreteRaftOutput × List ConcreteRaftPacket) :=
  RaftInputHandler run_state_machine input state
