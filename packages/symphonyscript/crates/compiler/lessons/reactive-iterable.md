# reactive-iterable

A placement `for`'s iterable is a compile-time value, and an iterable there that reads a cell is an error. (SPEC §7.9)

A placement `for` expands when the program compiles, so its iterations cannot wait on a cell; a `for` expression folds a cell's elements instead.

```sym-error
input x = 1
Clip c:
    for i in [x, x]:
        Note
```

The array reads the input `x`.

```sym
input x = 1
Clip c:
    for i in [1, 2]:
        Note/(60 + i + x)
```
