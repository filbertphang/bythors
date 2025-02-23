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
abbrev ConcreteRaftMessage := @MessageRaft ConcreteAddress ConcreteValue
abbrev ConcreteRaftData := @Data ConcreteAddress ConcreteValue ConcreteStateMachineData
abbrev ConcreteRaftInput := @Input ConcreteValue
abbrev ConcreteRaftOutput := @Output ConcreteValue
abbrev ConcreteRaftPacket := @Packet ConcreteAddress ConcreteRaftMessage

-- callback function for when consensus is reached on a log entry
-- this is identity, because we want to reach consensus on the input (for now).
def run_state_machine (v : ConcreteValue) (s : ConcreteStateMachineData)
  : ConcreteValue × ConcreteStateMachineData :=
  (v, s)

@[export raft_init]
def raft_init (me : ConcreteAddress) (nodes : List ConcreteAddress)
  : ConcreteRaftData :=
  init_handlers smdInit me nodes

@[export raft_handle_input]
def raft_handle_input
  (state : ConcreteRaftData)
  (clientId : ClientId)
  (value : ConcreteValue)
  : (ConcreteRaftData × List ConcreteRaftOutput × List ConcreteRaftPacket) :=
  let input := Input.ClientRequest clientId 0 value
  RaftInputHandler run_state_machine input state

@[export raft_handle_message]
def raft_handle_message
  (state : ConcreteRaftData)
  (src : ConcreteAddress)
  (msg : ConcreteRaftMessage)
  : (ConcreteRaftData × List ConcreteRaftOutput × List ConcreteRaftPacket) :=
  RaftNetHandler run_state_machine src msg state

@[export raft_check_output]
def raft_check_output
  (state : ConcreteRaftData)
  (index : ClientId)
  : Option ConcreteValue :=
  state.clientCache.find? index
  |> Option.map (λ (_, v) ↦ v)

-- convenience functions
@[export raft_create_entry]
def raft_create_entry
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

@[export raft_create_requestvote]
def raft_create_requestvote
  (term : Term)
  (candidateId : ConcreteAddress)
  (lastLogIndex : Index)
  (lastLogTerm : Term)
  : ConcreteRaftMessage := MessageRaft.RequestVote term candidateId lastLogIndex lastLogTerm

@[export raft_create_requestvotereply]
def raft_create_requestvotereply
  (term : Term)
  (voteGranted : Bool)
  : ConcreteRaftMessage := MessageRaft.RequestVoteReply term voteGranted

@[export raft_create_appendentriesreply]
def raft_create_appendentriesreply
  (term : Term)
  (leaderId : ConcreteAddress)
  (prevLogIndex : Index)
  (prevLogTerm : Term)
  (entries : List ConcreteRaftEntry)
  (leaderCommit : Index)
  : ConcreteRaftMessage := MessageRaft.AppendEntries term leaderId prevLogIndex prevLogTerm entries leaderCommit

@[export raft_create_appendentries]
def raft_create_appendentries
  (term : Term)
  (entries : List ConcreteRaftEntry)
  (success : Bool)
  : ConcreteRaftMessage := MessageRaft.AppendEntriesReply term entries success
