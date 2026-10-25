# state-syntax

A state is `state`, an optional depth `[N]`, a name, an optional type, `=` and its initializer; states are declared at top level. (SPEC §6.6)

A state is a cell of the whole program.

```sym-error
state = 1
```

The state has no name.

```sym
state count = 1
```
