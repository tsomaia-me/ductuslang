# wire-syntax

A wire placement is its kind, then optional flags, a name, a default value, a gate and an attribute clause, then `:` and its destination. (SPEC §8.1)

A wire connects its source to a destination.

```sym-error
Clip c:
    Plays
```

The wire has no destination.

```sym
Clip c:
    Note
    Plays: c
```
