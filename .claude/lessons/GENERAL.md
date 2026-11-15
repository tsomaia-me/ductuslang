# Lessons

Format: G-<ID>: Do X, because Y + cost when applicable.

Last ID: G-14

- G-1: Remove only a worktree you created, because another agent may be working in any other.
- G-2: Judge a check by its own exit code with nothing chained after it, because a trailing command masks the exit.
- G-3: Confirm a filtered test run ran the tests you meant, because a filter that matches nothing exits clean.
- G-4: Run a long check in the background with no outer timeout, because the timeout kills a healthy run.
- G-5: Wait on a run by its completion notice or process id, never a grep loop, because a run that ends another way never prints the line.
- G-6: Name the regression a test or guard catches before you propose deleting it, because it may be the only catch for that regression.
- G-7: Decide a call yourself when the owner's rulings, the documents and the code leave one answer, and cite them, because a question with one answer costs the owner a round trip.
- G-8: Bring the owner a call on product, design, money, law or anything irreversible that keeps two valid answers, because recorded as a default it ships a decision the owner never made.
- G-9: Search the documents for a term before you call something undecided, because a ruling recorded elsewhere gets asked twice.
- G-10: Delete what each run writes to a host when the run ends, because leftovers fill the disk and every later run fails.
- G-11: Kill a process by its pid, never `pkill -f` with a pattern, because the pattern matches the shell that runs it.
- G-12: Run the tests that read a document after you edit it and before you commit, because a reworded sentence turns them red.
- G-13: Call cargo by its full path in a command you run in the background, because a background shell may start without the profile that puts it on the path.
- G-14: Name the machine a target was set on, and take the time on that machine, before you compare the two, because a time from another machine reads as a pass or a failure that is neither.
