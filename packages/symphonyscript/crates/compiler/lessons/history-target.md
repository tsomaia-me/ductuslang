# history-target

A history read names an input, a state or a reactive `let`. (SPEC §6.6)

Only a cell has a past.

```sym-error
let a = 1
state s = a{1, 0}
```

`a` is a compile-time value.

```sym
input a = 1
state s = a{1, 0}
```
