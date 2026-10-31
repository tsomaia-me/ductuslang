# reactive-recursion

A call to a recursive function that reads a cell is an error, and its help says to write a `for` expression. (SPEC §6.3)

The compiler evaluates a recursive call to a value, and a call that reads a cell has no value until the program runs.

```sym-error
state held([]): i32[..10]
fn lowest(xs, i): if i >= xs.size then 127 else min(xs[i], lowest(xs, i + 1))
Clip c:
    Note/(lowest(held, 0))
```

`lowest` is recursive, and its argument reads the input `held`.

```sym
state held([]): i32[..10]
state lowest = for x in held into low from 127: min(low, x)
Clip c:
    Note/lowest
```

The `for` expression folds the held notes into one value and is one cell.
