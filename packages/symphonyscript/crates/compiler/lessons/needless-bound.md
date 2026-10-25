# needless-bound

A bound applies to a recursive function; a bound on a function that is not recursive is an error. (SPEC §6.3)

A function that never calls itself has no depth to bound.

```sym-error
fn f(x) bounded 3 else 0: x
```

`f` reaches itself through no call.

```sym
fn f(x): x
```
