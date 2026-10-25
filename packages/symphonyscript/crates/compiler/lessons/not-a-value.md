# not-a-value

A destination names an instance; it is never an inline placement or a pattern use. (SPEC §14)

A wire connects instances that exist elsewhere.

```sym-error
pattern V():
    Note
Clip c:
    Plays: V
```

`V` is a pattern.

```sym
Clip v:
    Note
Clip c:
    Note
    Plays: v
```
