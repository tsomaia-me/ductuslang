# does-not-fit

A value fits the type its use chooses. (SPEC §4.1)

A value that does not fit would change when stored.

```sym-error
Clip melody:
    Note/300000000000
```

The pitch is an `i32`, and the value does not fit it.

```sym
Clip melody:
    Note/60
```
