# unknown-suffix

A literal's suffix is a type suffix or a unit of the vocabulary; any other suffix is an error. (SPEC §2.6)

A suffix fixes a literal's type or unit, so an unknown one leaves both unknown.

```sym-error
let a = 2zz
```

`zz` is no type and no unit.

```sym
let a = 2bars
```
