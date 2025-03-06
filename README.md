# bythors

a rust crate for working with the raft protocol implemented in lean

## current status

> [!IMPORTANT]
> (branch raft-only):
>
> - this branch specializes the library to work with raft _only_
> - objective is to demonstrate that this whole protocol-in-lean-and-client-in-rust thing is (practically?) feasible
> - we ignore the whole part about letting users customize their own protocol and stuff
> - literally just a rust driver for raft

still somehow extremely WIP

**TODOs before the next milestone:**

- improve callback handling
- review all the other 'TODO's left in the code (of which there are a lot of)
- ensure no memory leaks
- benchmark (dsbugs and neobft and verdi-raft)
- correctness
- do (de/)serialization in lean so that we can just send raw bytes in rust -> no marshalling overhead?
  https://leanprover.zulipchat.com/#narrow/channel/113488-general/topic/Tree.20notation/near/271743211

- kv store is gonna be questionably implemented, proper way would be to:
  - achieve consensus over the log of events, i.e. client commands
  - need to change Value -> InputValue + OutputValue in Raft.lean
  - InputValue is an enum/inductive over GETrequest and PUTrequest
  - OutputValue is an enum over GETusccess/GETfailed/PUTsuccess/PUTfailed or something similar
  - state machine is the actual hash map that is maintained on each lean side
  - checkoutput does not have to check consensus, we just have to return the output of the command i guess?
