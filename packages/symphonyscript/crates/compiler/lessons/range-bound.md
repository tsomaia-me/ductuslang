# range-bound

The bounds of a `for` range are integers that fit `i64`. (SPEC §4.6)

A range counts whole steps.

```sym-error
Clip c:
    for i in 0..0.5:
        Note
```

`0.5` is no integer.

```sym
Clip c:
    for i in 0..4:
        Note
```
