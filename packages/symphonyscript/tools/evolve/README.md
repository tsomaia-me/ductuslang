# evolve

`evolve-probe` compiles hand-written programs and prints what each draws. Run it from `packages/symphonyscript` as `cargo run -q --manifest-path tools/evolve/Cargo.toml --bin evolve-probe -- [--budget <s>] [--jobs <n>] [--values] [--summary] <file>...`. A file holds programs split by lines `----`. A program's first line may be a tag choosing its vocabulary: `#example` (the default, SPEC §1.3), `#native`, `#bounded`, `#envelope`, `#bar`, `#signature` or `#to`; an unknown tag stops the run before any program compiles. A program's last line may be `#expect <codes>`, the codes it draws joined by spaces in the order the compiler reports them, nothing for a clean program, `panic` or `timeout`. Each program compiles as `main.sym` in its own process, `--jobs` at a time (default 4), and is killed past `--budget` seconds (default 30). Its block is a header `[<index>] #<tag> <file>:<line of its first source line>`, then a line `<code> <line>:<col>-<line>:<col> <summary>` per diagnostic, with `help: <help>` and its related spans indented below it, at positions counted from the program's first source line; `clean` when it draws nothing; its compile time in ms; `panic <message>` or `timeout` instead when it panics or passes the budget. `--values` adds `ok` or `differs: expected …, drew …` to each program with an `#expect` line, and `--summary` ends the run with the number of programs, clean programs, programs with an error, programs with a warning, panics and timeouts, and the `ok` and `differs` counts under `--values`. With this file as `cases.txt`, `evolve-probe --values --summary cases.txt` prints two blocks and `summary: programs 2, …, ok 2, differs 0`:

```
#signature
Clip c | three=([1]):
    Note
#expect
----
Clip c | three=([1]):
    Note
#expect no-entry-instance unknown-attribute
```

## The shared step

`python3 review/run.py shared <tip> [--into <dir>] [--against <commit>] [--exhaustive] [--stress] [--quiet-below <load>] [--jobs <n>]`, run from `tools/evolve`, is the head's shared step before a review (`docs/PLAN.md` § Reviews). It exports the tip, builds it in the `dev` profile, runs the four checks, each as a command of its own, the verdict diff and the speed figures, and writes `review-runs/<sha>/manifest.json` beside every log and result. The exhaustive tests run only under `--exhaustive`, after the unit tests, and the stress tests only under `--stress`, after them, as `docs/PLAN.md` § Work and checks says; the manifest's `ignored_tests` records whether each ran.

The verdict diff reads each program of `corpora/probes.jsonl.gz` as `evolve-probe` reads a file: the text splits at its `----` lines, a program's first-line tag chooses the vocabulary `evolve-check` checks it under, `#example` choosing `main`, and its `#expect` line is dropped. A probe whose tag names no vocabulary is not checked: it takes the verdict `unknown-tag`, and `diff.md` names it with its tag.

The speed figures, `figures.json`, are the step's last command. Each figure waits until the 1-minute load is below the `--quiet-below` load, 4 when the flag is absent and no wait under `inf`, the figures together waiting at most 10 minutes, records that load beside it, and is taken with one job: one `evolve-time` process that runs `Compiler::check` on its compile thread in the `dev` profile, one program at a time.

- A compile-time recursive call: a program whose `g` makes 1,000,000 distinct recursive calls over trees of distinct sizes, three runs, the median divided by 1,000,000.
- An inlined call: a program of 10,000 notes, each summing 100 calls `f(i, 0)` to `f(i, 99)` of a non-recursive `fn f(x, k): x + k` on an input, so Resolve and Infer each walk 1,000,000 distinct call sites, three runs, the median divided by 1,000,000.
- A level of nesting: the deep-expressions fixture at depth 100,000 for each construct alone and for all of them together, three runs each, the median divided by the levels.
- A program of the breaker corpus: every program of `corpora/breakers.jsonl.gz` checked once, with the verdict check's budget; a program the verdict check found past its budget is not run again, and it and a program past the budget here rank above every timed one; the median and the 99th percentile by nearest rank over every program.

Each per-unit figure counts the program's other text, its lexing and parsing among it, in its time.
