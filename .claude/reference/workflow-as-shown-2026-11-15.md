# The workflow, as shown to the owner on 2026-11-15

The peer's model of the work. Approved by the owner for testing on 2026-11-15, the owner's words in `owner-rulings-compiler-design.md`, seventh: once the design pass is over, the first piece of work tests it in a real case, with its numbers and its quality measured, so that a fault of the model is caught and fixed early. Approved for testing is not approved as final. The owner said of the reworded second principle "in a nutshell, sounds right-ish" and asked for "a simple textual mermaid diagram of the workflow (including me)", to make sure "that its as efficient as possible while ensuring the best possible quality as well" and that "there are not too much moving pieces". This is the diagram the peer showed in answer.

```mermaid
sequenceDiagram
    actor You
    participant Peer
    participant Tester
    participant Builder
    participant Reviewer
    participant Gate

    Note over You,Peer: Once, before any work starts
    You->>Peer: design the SPEC, the principles and the architecture together
    Peer->>Reviewer: each document, read cold
    Reviewer-->>Peer: faults, fixed
    Peer->>You: the documents
    You-->>Peer: approved

    Note over Peer,Gate: Then for every piece of work, without you
    Note over Reviewer: always a fresh agent that did not write what it reads
    Peer->>Peer: write the design page of the piece
    Peer->>Reviewer: the page, read cold
    Reviewer-->>Peer: faults, fixed, then the peer accepts the page
    par neither sees the work of the other
        Peer->>Tester: the page
        Tester-->>Peer: tests written from SPEC sentences
    and
        Peer->>Builder: the page
        Builder-->>Peer: plan of types and passes
    end
    Peer->>Reviewer: the plan against the design, the tests against the SPEC
    Reviewer-->>Peer: faults go back to their author, then the peer accepts
    Note over Tester,Builder: The tests are frozen. The builder cannot edit them.
    loop until green
        Builder->>Gate: code
        Gate-->>Builder: tests, cost, crashes, rule checks
    end
    par
        Peer->>Reviewer: read the code cold
    and
        Peer->>Tester: attack the build with new programs
    end
    Peer->>Reviewer: a second reader repeats every fault found
    Reviewer-->>Peer: the faults that hold up
    Peer->>Builder: those faults only, back to the gate loop
    Peer->>Peer: no fault left, merge to main

    Note over You,Gate: At any point, from any agent
    Builder->>Peer: a gap in the SPEC, or a design question
    alt the documents leave one answer
        Peer->>Peer: decide, write it down. A SPEC clarification is read cold, then committed.
    else more than one valid answer, or the design of the language would change
        Peer->>You: the question, the options, their costs, a pick
        You-->>Peer: your ruling, recorded
    end
    Peer->>You: results you can look at
```

What it holds: three kinds of agent beside the peer (a tester, a builder, a fresh reader), one machine gate, five checks a piece (the page read cold; the review before code; the gate; the code read cold with the tester's attack; the second reader). The owner stands at two places: the three documents before any work, and a question the documents leave with more than one valid answer or that would change the language's design.

Where it differs from the pushed agent drafts, which have seven agent files: the test reviewer's check is part of the one review before code; the prober's job is the tester's second pass over the built code; the verifier is a fresh reader that repeats each fault; the explorer is a helper and no step. None of this has run. The first small piece is to measure it in rounds, tokens and time before more is built on it. `STANDARD.md` and `FINDINGS.md` name four readers and say other things in places; their sentences follow once the owner has confirmed the workflow.
