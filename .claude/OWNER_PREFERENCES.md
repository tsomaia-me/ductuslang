# Owner preferences

One entry per preference: the rule in plain words.

- Notes hold what runs, what is owed and what is open; a line is removed once it stops being relevant.
- Lessons, preferences and notes are committed and pushed straight to `main`, staged by path, never forced.
- A builder never waits idle: no gap between a hand-back and the next launch, and no waiting on a check another process runs.
- Clean up what agents leave on the host without asking.
- No agent reads `.env` or prints a key.
- A choice that a cheap measurement would inform waits for the measurement; until then only the part that keeps every option open is taken.
