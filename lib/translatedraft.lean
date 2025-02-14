import LeanSts.State
import LeanSts.BFT.Network

open Std (HashMap)

deriving instance Repr for NetworkPacket

section Raft

variable {Address Value : Type}
variable [dec_addr : DecidableEq Address] [dec_value : DecidableEq Value]
variable [repr_addr : Repr Address] [repr_value : Repr Value]
variable [hashable_addr : Hashable Address]
variable [inhabited_value : Inhabited Value]

@[reducible] def ClientId := Nat
@[reducible] def InputId := Nat
@[reducible] def Term := Nat
@[reducible] def Index := Nat

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

end Raft
