# unit-mismatch

A unit value bound to an attribute of a different family, or of no family, is an error. (SPEC §3.6)

A unit says what a number measures, and an attribute measures one thing.

```sym-error
Clip c:
    Note | length=2s
```

`length` is measured in beats, and `2s` in time.

```sym
Clip c:
    Note | length=2bars
```
