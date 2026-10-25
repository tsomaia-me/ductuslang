# let-syntax

A `let` is `let`, a name, an optional type, `=` and its value or a `match`. (SPEC §6.2)

A `let` names a value.

```sym-error
let = 1
```

The `let` has no name.

```sym
let one = 1
```
