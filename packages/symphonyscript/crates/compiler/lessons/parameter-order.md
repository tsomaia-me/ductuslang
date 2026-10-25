# parameter-order

A parameter with a default follows every parameter without one. (SPEC §6.3)

A positional call omits only trailing parameters.

```sym-error
fn f(a = 1, b): a + b
```

`a` has a default and comes before `b`.

```sym
fn f(b, a = 1): a + b
```
