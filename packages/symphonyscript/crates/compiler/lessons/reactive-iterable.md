# reactive-iterable

An iterable that reads a cell is an error. (SPEC §7.9)

A `for` expands when the program compiles.

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
