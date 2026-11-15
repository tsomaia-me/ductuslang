# Running things

Read by every agent that runs a command.

## Hosts

- This host is slow and shared. wb2 (`ssh wb2`) is the reference machine and runs the gate.
- A time taken on this host is never compared with a target, because every target is stated for wb2. Growth across sizes on one host is comparable; a figure across hosts is not.
- Run nothing heavy on this host from 18:00Z to 02:00Z.

## Building and testing here

- Call cargo by its full path, `/home/cdagent/.cargo/bin/cargo`, under `nice -n 19`, with `-j 6`.
- Build unoptimised, `CARGO_PROFILE_DEV_OPT_LEVEL=0`, in a target directory of your own, `/home/cdagent/cargo-targets/<your task>/target`. A timing run and a run on a 1 MiB stack build in the `dev` profile as written.
- Run only what your role's file names, as a filtered run of the test binary that holds it, and confirm the output names the tests you meant, because a filter that matches nothing exits clean.
- Judge a run by its own exit code with nothing chained after it.
- Put a long run in the background with no outer timeout and wait on its completion notice or its process id.
- Kill a process by its pid, never by a pattern.

## Your place

- Work in the worktree your brief names, with `git -C <path>` and absolute paths. Remove only a worktree you created.
- Stage by path, `git add -- <path>`. Never `git add -A`, `git add .`, `git commit -a`, `git stash`, `git reset`, a rebase or a forced push.
- Keep scratch files under a folder named for your task in the scratchpad. Delete your target directory and your scratch when you hand back.
- Never read `.env` and never print a key.

## The gate's results

The gate writes one result a commit on wb2. Read the newest with the command your brief gives. A result names the commit, each command with its exit code, the count of rules passing and, for a refused commit, the path that refused it.
