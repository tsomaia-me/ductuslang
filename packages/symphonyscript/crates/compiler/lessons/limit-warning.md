# limit-warning

Every limit that draws an error has a warning value beside it, half the limit by default, and a count that passes the warning value draws a warning naming the call or the expansion; the warning stops nothing. (SPEC §4.6)

The warning tells you a program is halfway to a limit while it still compiles, so you can shrink the work or raise the limit before the error stops a build.

```sym-error
fn r(n): if n < 1 then 0 else r(n - 1)
Clip c:
    Note/(r(512))
```

The evaluation nests 513 calls of `r`, past the nesting depth's warning value of 512.

```sym
fn r(n): if n < 1 then 0 else r(n - 1)
Clip c:
    Note/(r(100))
```
