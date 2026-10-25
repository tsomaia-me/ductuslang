# condition-type

The condition of an `if`, a gate and a guard is a `bool`. (SPEC §5.4)

A condition chooses between two ways, so it is true or false.

```sym-error
let a = if 1 then 2 else 3
```

The condition is a number.

```sym
let a = if 1 > 0 then 2 else 3
```
