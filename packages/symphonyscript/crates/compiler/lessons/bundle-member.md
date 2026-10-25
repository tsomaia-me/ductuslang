# bundle-member

A bundle's members are placements without bodies, also when a pattern supplies them. (SPEC §7.8)

A bundle groups single placements.

```sym-error
pattern P():
    Plays: c
Clip c:
    [P]
```

The pattern's body is a wire.

```sym
pattern P():
    Note
Clip c:
    [P]
```
