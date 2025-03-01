import LeanSts.BFT.Network

open Lean (AssocList)
deriving instance Repr for NetworkPacket

-- adding some helpers for association lists to make life easier
namespace Lean.AssocList
  -- structure is effectively identical to List.hasDecEq
  -- see: https://github.com/leanprover/lean4/blob/93d4ae6635c0c755c9f7368f9b99483d4557b7a6/src/Init/Prelude.lean#L2328-L2339
  def hasDecEq
    {α : Type u} {β : Type v}
    [DecidableEq α] [DecidableEq β]
    : (a b : AssocList α β) → Decidable (a = b)
      | .nil, .nil => isTrue rfl
      | .cons _ _ _, .nil => isFalse (fun h => AssocList.noConfusion h)
      | .nil, .cons _ _ _ => isFalse (fun h => AssocList.noConfusion h)
      | .cons ka va as, .cons kb vb bs =>
        match decEq ka kb with
        | isFalse nkab => isFalse (fun h => AssocList.noConfusion h (fun hkab _ _ => absurd hkab nkab))
        | isTrue hkab => match decEq va vb with
          | isFalse nvab => isFalse (fun h => AssocList.noConfusion h (fun _ hvab _ => absurd hvab nvab))
          | isTrue hvab => match AssocList.hasDecEq as bs with
            | isFalse nsab => isFalse (fun h => AssocList.noConfusion h (fun _ _ hsab => absurd hsab nsab))
            | isTrue hsab => isTrue (hsab ▸ hvab ▸ hkab ▸ rfl)

  instance {α : Type u} {β : Type v}
    [DecidableEq α] [DecidableEq β]
    : DecidableEq (AssocList α β) := hasDecEq

  def getD [BEq α] (xs : AssocList α β) (a : α) (default : β): β :=
    match xs.find? a with
    | some b => b
    | none => default

  def update [BEq α] (xs : AssocList α β) (a : α) (f : Option β → β)
  : AssocList α β  :=
     xs.replace a (f (xs.find? a))
end Lean.AssocList

-- this implementation of Raft translated from verdi-raft
section Raft

-- type parameters and aliases

variable {Address Value StateMachineData  : Type}
variable [dec_addr : DecidableEq Address] [dec_value : DecidableEq Value] [dec_smd : DecidableEq StateMachineData]
variable [repr_addr : Repr Address] [repr_value : Repr Value]
variable [hashable_addr : Hashable Address]
variable [beq_addr : BEq Address]
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
deriving Repr, DecidableEq

local notation "RaftEntry" => (@Entry Address Value)

-- renamed to avoid name conflicts when linking,,,
inductive MessageRaft
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
    (entries : List RaftEntry)
    (leaderCommit : Index)
  | AppendEntriesReply
    (term : Term)
    (entries : List RaftEntry)
    (success : Bool)
deriving Repr, DecidableEq

local notation "RaftMessage" => (@MessageRaft Address Value)
local notation "RaftPacket" => (@Packet Address RaftMessage)

inductive Input
  | Timeout
  | ClientRequest
    (client : ClientId)
    (inputId : InputId)
    (input : Value)
deriving Repr, DecidableEq

inductive Output
  | NotLeader
    (client : ClientId)
    (inputId : InputId)
  | ClientResponse
    (client : ClientId)
    (inputId : InputId)
    (output : Value)
deriving Repr, DecidableEq

local notation "RaftInput" => (@Input Value)
local notation "RaftOutput" => (@Output Value)

-- cannot use `RaftOutput` here, because it gets treated as a
-- type parameter instead of the concrete type
-- (something with 'hygiene', and not substituting the local notation correctly?)
variable (callback : Value → StateMachineData → Value × StateMachineData)
variable (smdInit : StateMachineData)

inductive ServerType
  | Leader
  | Follower
  | Candidate
deriving Repr, DecidableEq

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
  stateMachine : StateMachineData

  -- (* leader state *)
  nextIndex :  AssocList Address Index
  matchIndex : AssocList Address Index
  shouldSend : Bool

  -- (* candidate state *)
  votesReceived : List Address

  -- (* whoami *)
  type : ServerType
  me : Address
  nodes : List Address

  -- (* map of client request to output value *)
  clientCache : AssocList ClientId (InputId × Value)

  -- (* ghost variables *)
  -- (omitted, because this should be for proofs only)
  -- electoralVictories : list (term * list name * list entry)
deriving DecidableEq

local notation "RaftData" => (@Data Address Value StateMachineData)


-- helper functions
def makePacket (src dst : Address) (msg : RaftMessage) : RaftPacket :=
  {
    src
    dst
    msg
    consumed := false
  }

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

def wonElection (nodes : List Address) (votes : List Address) : Bool :=
  nodes.length <= (2 * votes.length)

-- elections
def tryToBecomeLeader  (state : RaftData) :
  RaftData × List RaftOutput × List RaftPacket :=
  let nextTerm := state.currentTerm + 1
  let newState := {
    state with
    type := ServerType.Candidate
    votedFor := some state.me
    votesReceived := [state.me]
    currentTerm := nextTerm
  }
  let packets :=
    state.nodes
    |> List.filter (λ node ↦ node ≠ state.me)
    |> List.map (λ dstNode ↦ makePacket state.me dstNode (MessageRaft.RequestVote
      nextTerm
      state.me
      (maxIndex state.log)
      (maxTerm state.log)
    ))
  (newState, [], packets)

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
      }, MessageRaft.AppendEntriesReply term entries true)
    else
    ({
      nextState with
      type := ServerType.Follower
      leaderId := some leaderId
    }, MessageRaft.AppendEntriesReply term entries true)
  else
    match (findAtIndex state.log prevLogIndex) with
    | none =>
      (state, MessageRaft.AppendEntriesReply state.currentTerm entries false)
    | some entry =>
      if !(prevLogTerm = entry.eTerm) then
        (state, MessageRaft.AppendEntriesReply state.currentTerm entries false)
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
          }, MessageRaft.AppendEntriesReply term entries true)
        else
          ({
            nextState with
            type := ServerType.Follower
            leaderId := some leaderId
          }, MessageRaft.AppendEntriesReply term entries true)

def handleAppendEntriesReply
  (state : RaftData)
  (src : Address)
  (term : Term)
  (entries : List RaftEntry)
  (result : Bool)
  : RaftData × List RaftPacket :=
  if state.currentTerm = term then
    if result then
      let index := maxIndex entries
      let newMatchIndex :=
        state.matchIndex.update
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
    (state, MessageRaft.RequestVoteReply state.currentTerm false)
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
        }, MessageRaft.RequestVoteReply nextState.currentTerm true)
      | some candidateId' =>
        (nextState, MessageRaft.RequestVoteReply nextState.currentTerm (candidateId = candidateId'))
    else
      (nextState, MessageRaft.RequestVoteReply nextState.currentTerm false)

def handleRequestVoteReply
  (state : RaftData)
  (src : Address)
  (term : Term)
  (voteGranted : Bool)
  : RaftData :=
  if state.currentTerm < term then
    let nextState := advanceCurrentTerm state term
    {nextState with type := ServerType.Follower}
  else if state.currentTerm > term then
    state
  else
    let won := voteGranted && wonElection state.nodes state.votesReceived
    match state.type with
    | ServerType.Follower
    | ServerType.Leader => state
    | ServerType.Candidate =>
      let newVotesReceived := (match voteGranted with
        | true => [src]
        | false => []) ++ state.votesReceived
      let newType := match won with
        | true => ServerType.Leader
        | false => state.type
      let newMatchIndex := AssocList.empty.insert state.me (maxIndex state.log)
      {
        state with
        votesReceived := newVotesReceived
        type := newType
        matchIndex := newMatchIndex
        nextIndex := AssocList.empty
        -- electoralVictories omitted
      }

def handleMessageRaft
  (src : Address)
  (msg : RaftMessage)
  (state : RaftData)
  : RaftData × List RaftPacket :=
  match msg with
  | MessageRaft.AppendEntries term leaderId prevLogIndex prevLogTerm entries leaderCommit =>
    let (nextState, reply) := handleAppendEntries state term leaderId prevLogIndex prevLogTerm entries leaderCommit
    (nextState, [makePacket state.me src reply])

  | MessageRaft.AppendEntriesReply term entries result =>
    handleAppendEntriesReply state src term entries result

  | MessageRaft.RequestVote term _candidateId lastLogIndex lastLogTerm =>
    let (nextState, reply) := handleRequestVote state term src lastLogIndex lastLogTerm
    (nextState, [makePacket state.me src reply])

  | MessageRaft.RequestVoteReply term voteGranted =>
    (handleRequestVoteReply state src term voteGranted, [])

-- client handling
-- `callback` as an explicit argument here is a little weird,
-- but i guess we have to deal with it
def applyEntry
  (state : RaftData)
  (entry : RaftEntry)
  : RaftData × List Value :=
  let (output, newStateMachine) := callback (entry.eInput) (state.stateMachine)
  let newClientCache :=
    state.clientCache.insert entry.eClient (entry.eId, output)
  ({
    state with
    clientCache := newClientCache
    stateMachine := newStateMachine
  }, [output])

def cacheApplyEntry
  (state : RaftData)
  (entry : RaftEntry)
  : RaftData × List Value :=
  match state.clientCache.find? entry.eClient with
  | some (id, output) =>
    if entry.eId < id then
      (state, [])
    else
      if entry.eId = id then
        (state, [output])
      else
        applyEntry callback state entry
  | none =>
    applyEntry callback state entry

def applyEntries
  (node : Address)
  (state : RaftData)
  (entries : List RaftEntry)
  : RaftData × List RaftOutput :=
  entries.foldr
    (λ entry (st_acc, out_acc) ↦
      let (_st, out) := cacheApplyEntry callback state entry
      let out' :=
        if entry.eAt = node then
          out.map (λ output ↦ Output.ClientResponse entry.eClient entry.eId output)
        else
          []
      (st_acc, out' ++ out_acc)
    )
    (state, [])

def doGenericServer
  (node : Address)
  (state : RaftData)
  : RaftData × List RaftOutput × List RaftPacket :=
  let (state, out) := applyEntries callback node state
    (findGtIndex state.log state.lastApplied
     |> List.filter (λ entry ↦
       (state.lastApplied < entry.eIndex)
       && (entry.eIndex <= state.commitIndex))
     |> List.reverse)
  let newLastApplied := max state.commitIndex state.lastApplied
  ({state with lastApplied := newLastApplied}, out, [])

def replicaMessage
  (state : RaftData)
  (host : Address)
  : RaftPacket :=
  let prevIndex := (getNextIndex state host) - 1
  let prevTerm := match (findAtIndex state.log prevIndex) with
    | none => 0
    | some entry => entry.eTerm
  let newEntries := findGtIndex state.log prevIndex
  let msg := MessageRaft.AppendEntries state.currentTerm state.me prevIndex prevTerm newEntries state.commitIndex
  makePacket state.me host msg

def haveQuorum
  (state : RaftData)
  (N : Index)
  : Bool :=
  let nodesWithHigherMatchIndex := state.nodes.filter (λ h ↦ N ≤ state.matchIndex.getD h 0)
  2 * nodesWithHigherMatchIndex.length > state.nodes.length

def advanceCommitIndex (state : RaftData) : RaftData :=
  let entriesToCommit :=
    state.commitIndex
    |> findGtIndex state.log
    |> List.filter
        (λ entry ↦
          (state.currentTerm = entry.eTerm)
          && (state.commitIndex < entry.eIndex)
          && (haveQuorum state entry.eIndex))
  let newCommitIndex :=
    entriesToCommit
    |> List.map (λ entry ↦ entry.eIndex)
    |> List.foldl max state.commitIndex
  {state with commitIndex := newCommitIndex}

def doLeader
  (state : RaftData)
  : RaftData × List RaftOutput × List RaftPacket :=
  match state.type with
  | ServerType.Follower
  | ServerType.Candidate => (state, [], [])
  | ServerType.Leader =>
    let state' := advanceCommitIndex state
    match state'.shouldSend with
    | false => (state', [], [])
    | true =>
      let state'' := {state' with shouldSend := false}
      let replicaMessages :=
        state''.nodes
        |> List.filter (λ addr ↦ addr ≠ state''.me)
        |> List.map (replicaMessage state'')
      (state'', [], replicaMessages)

def RaftNetHandler
  (src : Address)
  (msg : RaftMessage)
  (state : RaftData)
  : (RaftData × List RaftOutput × List RaftPacket) :=
  let (state, pkts) := handleMessageRaft src msg state
  let (state', leaderOut, leaderPkts) := doLeader state
  let (state'', genericOut, genericPkts) := doGenericServer callback state'.me state'
  (state'', leaderOut ++ genericOut, pkts ++ leaderPkts ++ genericPkts)

def handleClientRequest
  (state : RaftData)
  (client : ClientId)
  (id : InputId)
  (input : Value)
  : (RaftData × List RaftOutput × List RaftPacket) :=
  match state.type with
  | ServerType.Follower
  | ServerType.Candidate => (state, [Output.NotLeader client id], [])
  | ServerType.Leader =>
    let index := maxIndex state.log + 1
    let newEntry : RaftEntry := {
      eAt := state.me
      eClient := client
      eId := id
      eIndex := index
      eTerm := state.currentTerm
      eInput := input
    }
    let newState := {
      state with
      log := (newEntry :: state.log)
      matchIndex := (state.matchIndex.replace state.me index)
      shouldSend := true
    }
    (newState, [], [])

def handleTimeout (state : RaftData)
  : (RaftData × List RaftOutput × List RaftPacket) :=
  match state.type with
  | ServerType.Follower
  | ServerType.Candidate => tryToBecomeLeader state
  | ServerType.Leader =>
    ({state with shouldSend := true}, [], [])

def handleInput (input : RaftInput) (state : RaftData)
  : (RaftData × List RaftOutput × List RaftPacket) :=
  match input with
  | Input.Timeout => handleTimeout state
  | Input.ClientRequest client id input => handleClientRequest state client id input

def RaftInputHandler (input : RaftInput) (state : RaftData)
  : (RaftData × List RaftOutput × List RaftPacket) :=
  let (state', handlerOut, pkts) := handleInput input state
  let (state'', leaderOut, leaderPkts) := doLeader state'
  let (state''', genericOut, genericPkts) := doGenericServer callback state''.me state''
  (state''' ,
  handlerOut ++ leaderOut ++ genericOut,
  pkts ++ leaderPkts ++ genericPkts)

def reboot (state : RaftData) : RaftData :=
  {
    state with
    nextIndex := AssocList.empty
    matchIndex := AssocList.empty
    shouldSend := false
    votesReceived := []
    type := ServerType.Follower
  }

def init_handlers (me : Address) (nodes : List Address) : RaftData :=
  {
    currentTerm := 0
    votedFor := none
    leaderId := none
    log := []
    commitIndex := 0
    lastApplied := 0
    stateMachine := smdInit
    nextIndex :=  AssocList.empty
    matchIndex := AssocList.empty
    shouldSend := false
    votesReceived := []
    type := ServerType.Follower
    me := me
    nodes := nodes
    clientCache := AssocList.empty
  }

end Raft
