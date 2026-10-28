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
