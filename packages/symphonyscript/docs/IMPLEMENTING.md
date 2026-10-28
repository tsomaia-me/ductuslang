# Implementing

How a SymphonyScript implementer runs a round, fixes an item and hands back. PLAN § How the work runs decides the commits, the checks, the progress file and the gaps; `SMELLS.md` holds the defects to write none of; a round's brief holds its items.

## The round

- Open no branch, pull request, Linear ticket, `.memory/tasks` state file or `pnpm` command, because the repository's root `CLAUDE.md` describes the platform's ticket flow, and this project commits on `main` (PLAN § Work and checks).
- Leave `packages/symphonyscript/Cargo.toml` and `tools/carry/` alone beside the files PLAN § Work and checks names, because the head owns them.
- Push only when you hand back with `STATUS: done`, with exactly `git -C /Users/torniketsomaia/projects/@tsomaia.tech/ductuslang push origin main:symphonyscript/progress`, alone and never forced, and report a refusal without trying another way, because a chained or retried push is how a wrong ref moves.
- Keep scratch files and break-once copies in the brief's scratch folder by absolute path, because the owner's uncommitted work shares the main checkout.
- Carry on past a memory push that prints `push blocked:`, because the next push carries what it held.
- Write a gap as `G<n>: <what the documents say, file and section>; <what is missing or in conflict>; <the options>`, because the head answers it from that line alone.
- Record a row the documents contradict as a gap and never adjust it, because a row is the head's reading of the documents, and an adjusted row hides a misreading from the one who made it.
- Before you call a red run a code fault, run the failing file alone and read the load, and run again a target reported failed whose every test printed `ok`, because other sessions' builds share the machine.

## How you fix an item

1. **Fix the class, not the case.** State the item's property in one sentence citing its SPEC or ARCHITECTURE section, and write in the progress file what its class varies: the construct's forms, the operators, the positions and the declaration orders, because a fix proven on the reported form leaves every other form of its class to the next review, a round each.
2. **Hold the class with a generated test.** Generate the class's programs over those dimensions, with expected values from the documents (a shape given by hand per form, or ARCHITECTURE §7's twins), as `tests/typing.rs` `every_fault_beside_another_fault_draws_by_the_placeholder_rule` generates every expression to depth 2 at five positions in every order, and name in the progress file any form the generator leaves out, because hand-listed rows hold the forms someone thought of, and a generator holds the forms the class has.
3. **Name every variant in a classifier.** Give a function that answers a question about a value by matching `ExprKind` or `Op` an arm for every variant and no catch-all arm (`SMELLS.md` § Classes), because a form nobody decided then fails to compile instead of answering "no".
4. **Count the cost in every shape.** Assert each walk you add or change at three doubling sizes, in every shape its input takes (a chain, a ring, nesting, sharing, many readers of one value, a growing argument) and in forward and reversed declaration order, because a walk is linear in the shapes its test builds and can be quadratic in the rest.
5. **Check that the fix changed no verdict outside its class.** Change no existing row's expected value the item does not name, and list in the progress file each one you change with the rule that changes it. Then build `evolve-check` (`tools/evolve`) from exports of the round's start and of your tip, run both over the programs of the brief's probes and of `tools/evolve/breakers/samples/` as one programs file, and explain in the progress file every verdict that differs outside the item's class, because a fix changes every program its rule reaches, and a regression shows in a program its own rows do not hold.
6. **Ask for the rule a class lacks.** When no SPEC or ARCHITECTURE sentence decides a class's verdicts, record one gap that names the class and proposes one rule before you fix rows, because verdicts decided a row at a time contradict one another, and only a stated rule makes them agree.

## Hand-back

End with this message and nothing else, because the head reads every hand-back in this form:

```text
STATUS: done | blocked
Commits: <sha> <title>, one per line
Checks: the four checks on <sha>: pass, or which failed
Items: each item: done (the test that shows it) | not done (why)
Gaps: each open G<n> line
Public surface: the lines added
Backup: pushed <sha> | not pushed (why)
Next: what remains
```

When the head resumes you, re-read your progress file before you continue, because the head's answer can change rows you built. Ignore a grounding note addressed to the head, because it describes the head's state, not yours.
