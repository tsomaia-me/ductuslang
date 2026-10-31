# history-offset

`x.past(k)` reads a value from the history of the cell `x`, and `k` is a positive integer known at compile time. (SPEC §6.6)

A cell's history is as deep as the deepest `past` read of it, which the compiler finds from the offsets.

```sym-error
state a(1)
state s = a.past(0)
```

The offset is 0.

```sym
state a(1)
state s = a.past(1)
```
