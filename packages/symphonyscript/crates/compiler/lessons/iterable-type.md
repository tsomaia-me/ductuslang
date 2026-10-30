# iterable-type

The iterable of a placement `for` and of a `for` expression is an array, element by element, or a range `a..b`, and nothing else; a range binds one name, and two names over it are an error. (SPEC §7.9)

A single value has no elements to iterate over, and a range has no positions apart from its values. `for i, k in 0..4` is an error too: write `for i in 0..4`.

```sym-error
Clip c:
    for i in 5:
        Note
```

`5` is a single value.

```sym
Clip c:
    for i in 0..5:
        Note
```
