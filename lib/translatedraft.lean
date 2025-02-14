import LeanSts.State
import LeanSts.BFT.Network

open Std (HashMap)

deriving instance Repr for NetworkPacket

namespace HashMap
  def update? {α : Type u} {β : Type v} [BEq α] [Hashable α] (hm : HashMap α β) (k : α) (f : Option β → β)
  : HashMap α β :=
    let v' := f (hm[k]?)
    hm.insert k v'
end HashMap

section Raft

-- type parameters and aliases

variable {Address Value : Type}
variable [dec_addr : DecidableEq Address] [dec_value : DecidableEq Value]
variable [repr_addr : Repr Address] [repr_value : Repr Value]
variable [hashable_addr : Hashable Address]
variable [inhabited_value : Inhabited Value]

@[reducible] def ClientId := Nat
@[reducible] def InputId := Nat
@[reducible] def Term := Nat
@[reducible] def Index := Nat

-- structures and other types

structure Entry :=
  eAt : Address
  eClient : ClientId
  eId : InputId
  eIndex : Index
  eTerm : Term
  eInput : Value
deriving Repr

local notation "RaftEntry" => (@Entry Address Value)

inductive Message
  | RequestVote
    (term : Term)
    (candidateId : Address)
    (lastLogIndex : Index)
    (lastLogTerm : Term)
  | RequestVoteReply
    (term : Term)
    (voteGranted : Bool)
  | AppendEntries
    (term : Term)
    (leaderId : Address)
    (prevLogIndex : Index)
    (prevLogTerm : Term)
    -- TODO: think about how to represent entries
    (entries : List RaftEntry)
    (leaderCommit : Index)
  | AppendEntriesReply
    (term : Term)
    (entries : List RaftEntry)
    (success : Bool)
deriving Repr

local notation "RaftMessage" => (@Message Address Value)

inductive Input
  | Timeout
  | ClientRequest
    (client : ClientId)
    (inputId : InputId)
    (input : Value)

inductive Output
  | NotLeader
    (client : ClientId)
    (inputId : InputId)
  | ClientResponse
    (client : ClientId)
    (inputId : InputId)
    (output : Value)

local notation "RaftInput" => (@Input Value)
local notation "RaftOutput" => (@Input Value)

inductive ServerType
  | Leader
  | Follower
  | Candidate
deriving Repr

structure Data :=
  -- (* persistent *)
  currentTerm : Term
  votedFor : Option Address
  leaderId : Option Address
  log : List RaftEntry
  -- (* volatile *)
  commitIndex : Index
  lastApplied : Index

  -- TODO: model stateMachine
  -- stateMachine : sorry

  -- (* leader state *)
  nextIndex :  HashMap Address Index
  matchIndex : HashMap Address Index
  shouldSend : Bool

  -- (* candidate state *)
  votesReceived : List Address

  -- (* whoami *)
  type : ServerType
  me : Address
  nodes : List Address

  -- (* client request state *)
  clientCache : List (ClientId × (InputId × RaftOutput))

  -- (* ghost variables *)
  -- (omitted, because this should be for proofs only)
  -- electoralVictories : list (term * list name * list entry)

local notation "RaftData" => (@Data Address Value dec_addr hashable_addr)

-- helper functions
def findAtIndex (entries : List RaftEntry) (i : Index) : Option RaftEntry :=
  List.find? (λ entry ↦ entry.eIndex = i) entries

def findGtIndex (entries : List RaftEntry) (i : Index) : List RaftEntry :=
  List.filter (λ entry ↦ entry.eIndex > i) entries

def removeAfterIndex (entries : List RaftEntry) (i : Index) : List RaftEntry :=
  List.filter (λ entry ↦ entry.eIndex <= i) entries

def maxIndex (entries : List RaftEntry) : Index :=
  let maxOpt :=
    List.map (λ entry ↦ entry.eIndex) entries
    |> List.maximum?
  Option.getD maxOpt 0

def maxTerm (entries : List RaftEntry) : Index :=
  let maxOpt :=
    List.map (λ entry ↦ entry.eTerm) entries
    |> List.maximum?
  Option.getD maxOpt 0

def advanceCurrentTerm (state : RaftData) (newTerm : Term) : RaftData :=
  if newTerm > state.currentTerm then
    {
      state with
      currentTerm := newTerm
      votedFor := none
      type := ServerType.Follower
      leaderId := none
    }
  else
    state

def getNextIndex (state : RaftData) (address : Address) : Index :=
  state.nextIndex.getD address (maxIndex state.log)

def haveNewEntries
  (state : RaftData)
  (entries : List RaftEntry)
  : Bool :=
  and
  (!List.isEmpty entries)
  (match findAtIndex state.log (maxIndex entries) with
    | none => true
    | some entry => !(maxTerm entries = entry.eTerm))

def moreUpToDate (p1 p2 : Term × Index) : Bool :=
  let (t1, i1) := p1
  let (t2, i2) := p2
  (t1 > t2) || ((t1 = t2) && (i1 >= i2))

-- elections
def tryToBecomeLeader  (state : RaftData) :
  List RaftOutput × RaftData × List (Address × RaftMessage) :=
  let nextTerm := state.currentTerm + 1
  let newState := {
    state with
    type := ServerType.Candidate
    votedFor := some state.me
    votesReceived := [state.me]
    currentTerm := nextTerm
  }
  -- TODO: replace packet implementation with LeanSts Packet
  -- (because that's how we implement it on the rust side)
  let packets :=
    state.nodes
    |> List.filter (λ node ↦ node ≠ state.me)
    |> List.map (λ node ↦ (node,
      Message.RequestVote
      nextTerm
      state.me
      (maxIndex state.log)
      (maxTerm state.log)
    ))
  ([], newState, packets)

-- message handlers
def handleAppendEntries
  (state : RaftData)
  (term : Term)
  (leaderId : Address)
  (prevLogIndex : Index)
  (prevLogTerm : Term)
  (entries : List RaftEntry)
  (leaderCommit : Index)
  : RaftData × RaftMessage :=
  let nextState := advanceCurrentTerm state term
  if prevLogIndex = 0 then
    if (haveNewEntries state entries) then
      let newCommitIndex :=
        max state.commitIndex (min leaderCommit (maxIndex entries))
      ({
        nextState with
        log := entries
        commitIndex := newCommitIndex
      }, Message.AppendEntriesReply term entries true)
    else
    ({
      nextState with
      type := ServerType.Follower
      leaderId := some leaderId
    }, Message.AppendEntriesReply term entries true)
  else
    match (findAtIndex state.log prevLogIndex) with
    | none =>
      (state, Message.AppendEntriesReply state.currentTerm entries false)
    | some entry =>
      if !(prevLogTerm = entry.eTerm) then
        (state, Message.AppendEntriesReply state.currentTerm entries false)
      else
        if haveNewEntries state entries then
          let log' := removeAfterIndex state.log prevLogIndex
          let log'' := entries ++ log'
          ({
            nextState with
            log := log''
            commitIndex := max state.commitIndex (min leaderCommit (maxIndex log''))
            type := ServerType.Follower
            leaderId := some leaderId
          }, Message.AppendEntriesReply term entries true)
        else
          ({
            nextState with
            type := ServerType.Follower
            leaderId := some leaderId
          }, Message.AppendEntriesReply term entries true)

def handleAppendEntriesReply
  (state : RaftData)
  (src : Address)
  (term : Term)
  (entries : List RaftEntry)
  (result : Bool)
  -- TODO: impl with packets too
  : RaftData × List (Address × RaftMessage) :=
  if state.currentTerm = term then
    if result then
      let index := maxIndex entries
      let newMatchIndex :=
        HashMap.update?
        state.matchIndex
        src
        (λ idxOpt ↦ max (idxOpt.getD 0) index)
      let newNextIndex :=
        state.nextIndex.insert
        src
        (max (getNextIndex state src) (index + 1))
      ({
        state with
        matchIndex := newMatchIndex
        nextIndex := newNextIndex
      }, [])
    else
      let newNextIndex :=
        state.nextIndex.insert
        src
        ((getNextIndex state src) - 1)
      ({
        state with
        nextIndex := newNextIndex
      }, [])
  else if state.currentTerm < term then
    -- leader behind, convert to follower
    (advanceCurrentTerm state term, [])
  else
    -- follower behind, ignore
    (state, [])

def handleRequestVote
  (state : RaftData)
  (term : Term)
  (candidateId : Address)
  (lastLogIndex : Index)
  (lastLogTerm : Term)
  : RaftData × RaftMessage :=
  if state.currentTerm > term then
    (state, Message.RequestVoteReply state.currentTerm false)
  else
    let nextState := advanceCurrentTerm state term
    if (nextState.leaderId.isNone)
      && (moreUpToDate (lastLogTerm, lastLogIndex) ((maxTerm nextState.log), (maxIndex nextState.log)))
    then
      match nextState.votedFor with
      | none =>
        ({
          nextState with
          votedFor := some candidateId
        }, Message.RequestVoteReply nextState.currentTerm true)
      | some candidateId' =>
        (nextState, Message.RequestVoteReply nextState.currentTerm (candidateId = candidateId'))
    else
      (nextState, Message.RequestVoteReply nextState.currentTerm false)

end Raft
