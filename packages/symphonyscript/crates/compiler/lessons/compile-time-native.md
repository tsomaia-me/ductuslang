# compile-time-native

A native function with a string parameter or result runs only at compile time, and a call to one with an argument that reads a cell is an error. (SPEC §10.1)

Strings exist only at compile time.

```sym-error natives
input i = 0
Clip c:
    Note/(text_length(i))
```

The argument reads the input `i`.

```sym natives
Clip c:
    Note/(text_length("abc"))
```
