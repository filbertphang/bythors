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

-- instantiate state machine as the identity machine
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
-- this is identity, because we want to reach consensus on the input (for now).
def run_state_machine (v : ConcreteValue) (s : ConcreteStateMachineData)
  : ConcreteValue × ConcreteStateMachineData :=
  (v, s)

@[export init]
def init (me : ConcreteAddress) (nodes : List ConcreteAddress)
  : ConcreteRaftData :=
  init_handlers smdInit me nodes

@[export handle_message]
def handle_message
  (state : ConcreteRaftData)
  (src : ConcreteAddress)
  (msg : ConcreteRaftMessage)
  : (ConcreteRaftData × List ConcreteRaftOutput × List ConcreteRaftPacket) :=
  RaftNetHandler run_state_machine src msg state

@[export handle_input]
def handle_input
  (state : ConcreteRaftData)
  (input : ConcreteRaftInput)
  : (ConcreteRaftData × List ConcreteRaftOutput × List ConcreteRaftPacket) :=
  RaftInputHandler run_state_machine input state

@[export check_output]
def check_output
  (state : ConcreteRaftData)
  (index : ClientId)
  : Option ConcreteValue :=
  state.clientCache.find? index
  |> Option.map (λ (_, v) ↦ v)

@[export create_entry]
def create_entry
  (eAt : ConcreteAddress)
  (eClient : ClientId)
  (eId : InputId)
  (eIndex : Index)
  (eTerm : Term)
  (eInput : ConcreteValue)
  : ConcreteRaftEntry :=
  {
    eAt
    eClient
    eId
    eIndex
    eTerm
    eInput
  }
