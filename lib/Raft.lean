import LeanSts.State
import LeanSts.BFT.Network

-- define a opaque Repr for functions, so that we can Eval things.
instance (α : Type) (β : Type) : Repr (α → β) where
  reprPrec _ _ := Std.Format.text "(function, opaque)"

/-
  simple un-optimized implementation, excludes cluster
  membership changes and log compaction
-/
section Raft

variable {Address Value : Type}
variable [dec_addr : DecidableEq Address] [dec_value : DecidableEq Value]
variable [repr_addr : Repr Address] [repr_value : Repr Value]

@[reducible] def Term := Nat
@[reducible] def Index := Nat

-- TODO: make use of this
inductive Mode
  | Leader
  | Follower
  | Candidate
deriving Repr

inductive Message
  | NewClientEntry
    (entry : Value)
  | AppendEntries
    (term : Term)
    (leaderId : Address)
    (prevLogIndex : Index)
    (prevLogTerm : Term)
    -- TODO: think about how to represent entries
    (entries : List (Term × Value))
    (leaderCommit : Index)
  | AppendEntriesReply
    (term : Term)
    (fromId : Address)
    (success : Bool)
  | RequestVote
    (term : Term)
    (candidateId : Address)
    (lastLogIndex : Index)
    (lastLogTerm : Term)
  | RequestVoteReply
    (term : Term)
    (fromId : Address)
    (voteGranted : Bool)
deriving Repr

structure NodeState :=
  /- persistent state on leaders -/
  id : Address
  leaderId : Option Address
  allNodes : List Address
  currentTerm : Term
  votedFor : Option Address
  -- TODO: type for `log`?
  log : List (Term × Value)

  /- volatile state on servers -/
  mode : Mode
  commitIndex : Index

  /- ignore `lastApplied` temporarily, because we do not
     specify callbacks yet to handle applying a command to
     the state machine.

     callbacks can probably be implemented as an `[extern]` function of type `Value ↦ unit` or something similar
     (but then it needs to be linked properly)
   -/
  -- lastApplied : Index

  /- volatile state on leaders -/
  /- (re-initialized after election) -/
  nextIndex : Address → Index
  matchIndex : Address → Index
deriving Repr

local notation "RaftMessage" => (@Message Address Value)
local notation "RaftState" => (@NodeState Address Value)
local notation "RaftPacket" => (Packet Address RaftMessage)

def makePacket (state : RaftState) (dst : Address) (msg: RaftMessage) : RaftPacket :=
  {
    src := state.id
    dst
    msg
    consumed := false
  }

def initLocalState
  (id : Address)
  (leaderId : Address)
  (nodes : List Address)
  : RaftState :=
  {
    id := id
    leaderId := leaderId
    allNodes := nodes
    currentTerm := 0
    votedFor := none
    log := []

    mode := if id = leaderId then Mode.Leader else Mode.Follower

    commitIndex := 0
    -- lastApplied := 0

    nextIndex := λ _ ↦ 1
    matchIndex := λ _ ↦ 0
  }

-- follower (receiver) implementation
def handleAppendEntries
    (state : RaftState)
    (term : Term)
    (leaderId : Address)
    (prevLogIndex : Index)
    (prevLogTerm : Term)
    -- assume entries are sorted in order of ascending index
    (entries : List (Term × Value))
    (leaderCommit : Index)
    : RaftState × List RaftPacket :=
    let reply (success : Bool) : List RaftPacket :=
      [makePacket state leaderId (Message.AppendEntriesReply term state.id success)]
    if term < state.currentTerm then
      (state, reply false)
    else
      match List.get? state.log prevLogIndex with
      | none =>
        -- section 5.1
        (state, reply false)
      | some (entryTerm, _) =>
        -- section 5.3
        if entryTerm = prevLogTerm then
          let newLog := List.take prevLogIndex state.log
          let newState := {state with log := newLog}
          (newState, reply false)
        else
          let baseLog := List.take (prevLogIndex + 1) state.log
          let newLog := List.append baseLog entries
          let newCommitIndex := max (state.commitIndex) (min leaderCommit (List.length newLog - 1))
          let newState := {
            state with
            log := newLog,
            commitIndex := newCommitIndex
          }
          (newState, reply true)

def handleAppendEntriesReply
    (state : RaftState)
    (term : Term)
    (fromId : Address)
    (success : Bool)
    : RaftState × List RaftPacket :=
    match success with
    | true =>
      -- TODO: update matchIndex and nextIndex for follower?
      sorry
    | false =>
      let decrementedNextIndex := (state.nextIndex fromId) - 1
      let newState := { state with nextIndex := state.nextIndex[fromId ↦ decrementedNextIndex]}
      let prevLogIndex := decrementedNextIndex - 1
      let logLength := List.length state.log
      let ltProp := prevLogIndex < logLength
      if ltProof : ltProp then
        let prevLogFin : Fin (List.length state.log) := Fin.mk prevLogIndex ltProof
        let retryMsg :=
          Message.AppendEntries
          state.currentTerm
          state.id
          prevLogIndex
          (Prod.fst (List.get state.log prevLogFin))
          (List.drop (prevLogIndex + 1) state.log)
          state.commitIndex
        (newState, [makePacket state fromId retryMsg])
      else
        -- error: prevLogIndex should always be < logLength
        sorry

-- follower (receiver) implementation
def handleRequestVote
    (state : RaftState)
    (term : Term)
    (candidateId : Address)
    (lastLogIndex : Index)
    (lastLogTerm : Term)
    -- : Term × Bool :=
    : RaftState × List RaftPacket :=
    sorry

def handleMessage
  (state : RaftState)
  (msg : RaftMessage)
  : RaftState × List RaftPacket :=
  match msg with
  | Message.AppendEntries term leaderId prevLogIndex prevLogTerm entries leaderCommit =>
    handleAppendEntries state term leaderId prevLogIndex prevLogTerm entries leaderCommit
  | Message.AppendEntriesReply term fromId success =>
    handleAppendEntriesReply state term fromId success
  | Message.RequestVote term candidateId lastLogIndex lastLogTerm =>
    handleRequestVote state term candidateId lastLogIndex lastLogTerm
  -- todo: handle request vote replies
  | _ => sorry

end Raft

section RaftTest
  -- putting this in the same file, for now
  @[reducible] def ConcreteAddress := Nat
  @[reducible] def ConcreteValue := String

  abbrev ConcreteRaftMessage := (@Message ConcreteAddress ConcreteValue)
  abbrev ConcreteRaftState := (@NodeState ConcreteAddress ConcreteValue)
  abbrev ConcreteRaftPacket := (Packet ConcreteAddress ConcreteRaftMessage)
  -- abbrev ConcreteRaftProtocol := @NetworkProtocol ConcreteAddress (ConcreteRBMessage) (ConcreteRBState) (ConcreteRBInternalTransition)

  def all_nodes : List ConcreteAddress := [0, 1, 2, 3]
  def nodes : List ConcreteRaftState := List.map (λ addr ↦ initLocalState addr 0 all_nodes) all_nodes

  #eval nodes

end RaftTest
