# unknown-name

Every name resolves to a declaration in scope. (SPEC §6.5)

A name that names nothing has no value.

```sym-error
let a = b
```

`b` is declared nowhere.

```sym
let b = 1
let a = b
```
