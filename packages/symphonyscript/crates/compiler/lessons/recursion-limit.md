# recursion-limit

A compilation evaluates at most 1,000,000 distinct recursive calls. (SPEC §4.6)

The limit keeps a recursion with many distinct arguments from running without end.

```sym-error
fn f(a, b, c): if a < 1 then 0 else f(a - 1, b + 1, c) + f(a - 1, b, c + 1) + f(a - 1, b, c)
Clip k:
    Note/(f(200, 0, 0))
```

The calls differ in `b` and `c` and number far more than a million.

```sym
fn f(a): if a < 1 then 0 else f(a - 1) + 1
Clip k:
    Note/(f(200))
```
