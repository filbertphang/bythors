import LeanSts.BFT.Network

/-
  simple un-optimized implementation, excludes cluster
  membership changes and log compaction
-/
section Raft

variable {Address Round Value : Type}

@[reducible] def Term := Nat
@[reducible] def Index := Nat

inductive Message
  | AppendEntries
    (term : Term)
    (leaderId : Address)
    (prevLogIndex : Index)
    (prevLogTerm : Term)
    -- TODO: think about how to represent entries
    (entries : List Value)
    (leaderCommit : Index)
  | RequestVote
    (term : Term)
    (candidateId : Address)
    (lastLogIndex : Index)
    (lastLogTerm : Term)

structure NodeState :=
  /- persistent state on leaders -/
  id : Address
  allNodes : List Address
  currentTerm : Term
  votedFor : Term → Option Address
  -- TODO: type for `log`?
  log : Round → Option Value

  /- volatile state on servers -/
  commitIndex : Index
  lastApplied : Index

  /- volatile state on leaders -/
  /- (re-initialized after election) -/
  nextIndex : Address -> Index
  matchIndex : Address -> Index

local notation "RaftMessage" => (@Message Address Value)
local notation "RaftState" => (@NodeState Address Round Value)
local notation "RaftPacket" => (Packet Address RaftMessage)

def initLocalState
  (id : Address)
  (nodes : List Address)
  : RaftState :=
  {
    id := id
    allNodes := nodes
    currentTerm := 0
    votedFor := λ _ ↦ none
    log := λ _ ↦ none

    commitIndex := 0
    lastApplied := 0

    nextIndex := λ _ ↦ 1
    matchIndex := λ _ ↦ 0
  }

-- receiver implementation
def handleAppendEntries
    (term : Term)
    (leaderId : Address)
    (prevLogIndex : Index)
    (prevLogTerm : Term)
    (entries : List Value)
    (leaderCommit : Index)
    : Term × Bool :=
    sorry

-- receiver implementation
def handleRequestVote
    (term : Term)
    (candidateId : Address)
    (lastLogIndex : Index)
    (lastLogTerm : Term)
    : Term × Bool :=
    sorry

end Raft
