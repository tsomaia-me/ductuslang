# argument-count

A call passes each parameter that has no default. (SPEC §5.7)

A missing argument leaves its parameter without a value, and an extra one binds nothing.

```sym-error
let a = lerp(1, 2)
```

`lerp` takes three arguments.

```sym
let a = lerp(1, 2, 0.5)
```
