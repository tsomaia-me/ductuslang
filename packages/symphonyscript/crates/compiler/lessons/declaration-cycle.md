# declaration-cycle

Declarations whose values read one another in a cycle are an error naming the cycle; a state's history reads form no cycle. (SPEC §6.2)

A value that depends on itself has no first value to compute.

```sym-error
let a = b + 1
let b = a + 1
```

`a` reads `b`, and `b` reads `a`.

```sym
let a = 1
let b = a + 1
```
