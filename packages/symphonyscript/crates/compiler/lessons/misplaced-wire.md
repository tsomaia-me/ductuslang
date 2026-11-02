# misplaced-wire

Wires and bundles are not bundle members. (SPEC §7.8)

A bundle groups placements of nodes.

```sym-error
Plant p:
    Valve as v
    Controller:
        [Drives: v]
```

The member is a wire.

```sym
Plant p:
    Valve as v
    Controller:
        Drives: v
```
