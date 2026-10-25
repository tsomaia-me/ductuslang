# match-syntax

A `match` names its scrutinee, ends its line with `:` and lists its arms indented under it. (SPEC §5.4)

The arms are what the `match` chooses among.

```sym-error
input b = 1
let a = match b
```

The `match` has no `:` and no arms.

```sym
input b = 1
let a = match b:
    0: 10
    else: 20
```
