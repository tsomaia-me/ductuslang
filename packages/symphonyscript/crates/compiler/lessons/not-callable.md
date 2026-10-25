# not-callable

A call invokes a built-in, a prelude function, a native function or a source function. (SPEC §5.7)

Only functions have parameters to bind.

```sym-error
let a = 1(2)
```

`1` is no function.

```sym
let a = abs(2)
```
