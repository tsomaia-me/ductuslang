# Owner preferences

One entry per preference: the rule in plain words.

- Notes hold what runs, what is owed and what is open; a line is removed once it stops being relevant.
- Approved lessons and preferences, and notes, are committed and pushed straight to `main`, staged by path, never forced.
- A builder never waits idle: no gap between a hand-back and the next launch, and no waiting on a check another process runs.
- Clean up what agents leave on the host without asking.
- No agent reads `.env` or prints a key.
- The code rules are self-contained: an agent learns how to write good code from them alone, never from existing code, and no rule leaves the builder to judge what is good.
- A session that ends leaves three things: notes that bring a fresh session to the current state (what is decided, what is being discussed, what is still to plan, design and decide, each with the path of what to read), a short handoff prompt, and any message worth keeping as it was said, saved word for word in a file the notes name.
- When the peer relays a designer's message, it puts a short "Key points" list after its reasoning and before its draft reply: one point a bullet, short plain sentences, and every numbered or lettered item (a line such as 5.2, a choice such as J, an owed item) said in one sentence where it is named, so the list reads without looking anything up.
- An open item for the owner is written in plain language, with what the owner needs to answer it without opening a transcript or another file: what is asked, why it is open, and, where there are choices, each with its cost and the one preferred and why; never a bare label, never an essay.
- Text proposed for a file — a lesson, a preference, a SPEC sentence, a skill rule, or an entry for the message catalogue among it — is shown to the owner in the chat in a quote block of its own, with the exact line it replaces when it refines or widens one and the exact line itself when it removes one, never a bare ID or line number, and is written only after the owner approves it.
- A question from the owner comes first. Where it needs discussion and not a one-off answer, the peer answers it and starts no further action until the owner settles it. Where it is a simple question with a one-shot answer, such as one about what is settled, the state of things or a fact, the peer answers it first and then goes on with what it was doing. In both cases what is already running keeps running, the peer may read, fetch or check what a grounded answer needs, and it keeps recording the owner's words and its notes.
