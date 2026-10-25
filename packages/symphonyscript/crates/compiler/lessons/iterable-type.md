# iterable-type

A `for` iterates over a range or an array of scalars, enum values, strings or node references. (SPEC §7.9)

A single value has no elements to iterate over.

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
