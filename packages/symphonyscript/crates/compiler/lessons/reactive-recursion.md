# reactive-recursion

A call to a recursive function that reads a cell is an error, and its help says to write a `for` expression. (SPEC §6.3)

The compiler evaluates a recursive call to a value, and a call that reads a cell has no value until the program runs.

```sym-error
state readings([]): i32[..10]
fn lowest(xs, i): if i >= xs.size then 100 else min(xs[i], lowest(xs, i + 1))
Plant p:
    Valve/(lowest(readings, 0))
```

`lowest` is recursive, and its argument reads the input `readings`.

```sym
state readings([]): i32[..10]
state lowest = for x in readings into low from 100: min(low, x)
Plant p:
    Valve/lowest
```

The `for` expression folds the readings into one value and is one cell.
