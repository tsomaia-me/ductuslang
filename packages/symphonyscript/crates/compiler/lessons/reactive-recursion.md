# reactive-recursion

A call to a recursive function that reads a cell is an error, and its help says to write a `for` expression. (SPEC §6.3)

The compiler evaluates a recursive call to a value, and a call that reads a cell has no value until the program runs.

```sym-error
input i = 1
fn f(n): if n < 1 then 0 else f(n - 1)
Clip c:
    Note/(f(i))
```

`f` is recursive, and its argument reads the input `i`.

```sym
fn f(n): if n < 1 then 0 else f(n - 1)
Clip c:
    Note/(f(1))
```
