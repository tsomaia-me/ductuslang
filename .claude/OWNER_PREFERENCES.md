# Owner preferences

One entry per preference: the rule in plain words.

- Notes hold what runs, what is owed and what is open; a line is removed once it stops being relevant.
- Lessons, preferences and notes are committed and pushed straight to `main`, staged by path, never forced.
- A builder never waits idle: no gap between a hand-back and the next launch, and no waiting on a check another process runs.
- Clean up what agents leave on the host without asking.
- No agent reads `.env` or prints a key.
- The code rules are self-contained: an agent learns how to write good code from them alone, never from existing code, and no rule leaves the builder to judge what is good.
- A session that ends leaves three things: notes that bring a fresh session to the current state (what is decided, what is being discussed, what is still to plan, design and decide, each with the path of what to read), a short handoff prompt, and any message worth keeping as it was said, saved word for word in a file the notes name.
- When the peer relays a designer's message, it puts a short "Key points" list after its reasoning and before its draft reply: one point a bullet, short plain sentences, and every numbered or lettered item (a line such as 5.2, a choice such as J, an owed item) said in one sentence where it is named, so the list reads without looking anything up.
