# history-target

A history read names a cell: a `state`, a `stream` or a `channel`, host-fed or derived. (SPEC §6.6)

Only a cell has a past and a step; a compile-time value never steps.

```sym-error
let a = 1
state s = a.past(1, 0)
```

`a` is a compile-time value.

```sym
state a(1)
state s = a.past(1, 0)
```
