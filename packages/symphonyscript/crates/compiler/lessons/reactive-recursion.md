# reactive-recursion

A call that reads a cell, to a recursive function that is not bounded or that shares a cycle of calls with one that is not, is an error. (SPEC §6.3)

Such a call runs at runtime, and only a bound gives it a fixed size there.

```sym-error
input i = 1
fn f(n): if n < 1 then 0 else f(n - 1)
Clip c:
    Note/(f(i))
```

`f` has no bound, and its argument reads the input `i`.

```sym
input i = 1
fn f(n) bounded 8 else 0: if n < 1 then 0 else f(n - 1)
Clip c:
    Note/(f(i))
```
