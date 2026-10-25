# misplaced-wire

Wires and bundles are not bundle members. (SPEC §7.8)

A bundle groups placements of nodes.

```sym-error
Clip c:
    [Plays: c]
```

The member is a wire.

```sym
Clip c:
    Note
    Plays: c
```
