# reactive-iterable

A placement `for`'s iterable is a compile-time value, and an iterable there that reads a cell is an error. (SPEC §7.9)

A placement `for` expands when the program compiles, so its iterations cannot wait on a cell; a `for` expression folds a cell's elements instead.

```sym-error
state x(0.5)
Plant p:
    for i in [x, x]:
        Valve
```

The array reads the input `x`.

```sym
state x(0.5)
Plant p:
    for i in [0.1, 0.2]:
        Valve/(i + x)
```
