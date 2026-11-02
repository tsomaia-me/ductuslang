# reactive-let

A top-level `let` or a pattern's `let` that reads a cell, directly or through other declarations, is an error. (SPEC §6.2)

A `let` names a compile-time value, computed once when the program compiles; a value that changes while the graph runs is a cell, and a derived `state` computes it.

```sym-error
state level(0.5)
let brighter = level + 0.2
```

`brighter` reads the cell `level`.

```sym
state level(0.5)
state brighter = level + 0.2
```
