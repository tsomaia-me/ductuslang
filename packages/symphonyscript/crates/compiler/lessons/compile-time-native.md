# compile-time-native

A native function with a string parameter or result runs only at compile time, and a call to one with an argument that reads a cell is an error. (SPEC §10.1)

Strings exist only at compile time.

```sym-error natives
state n(0)
Clip c:
    Note/(tag("abc", n))
```

`tag` takes a string, and its second argument reads the input `n`.

```sym natives
Clip c:
    Note/(tag("abc", 2))
```
