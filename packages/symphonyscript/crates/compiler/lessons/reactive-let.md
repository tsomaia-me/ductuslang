# reactive-let

A top-level `let` or a pattern's `let` that reads a cell, directly or through other declarations, is an error. (SPEC §6.2)

A `let` names a compile-time value, computed once when the program compiles; a value that changes while the graph runs is a cell, and a derived `state` computes it.

```sym-error
state note(60)
let louder = note + 12
```

`louder` reads the cell `note`.

```sym
state note(60)
state louder = note + 12
```
