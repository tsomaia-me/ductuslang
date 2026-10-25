# unreachable-instance

A top-level instance that no entry-kind instance reaches, through children and wires, draws a warning. (SPEC §10.2)

Nothing plays an instance that the entry never reaches.

```sym-error
Session s:
    Track
Clip lonely:
    Note
```

No instance reaches `lonely`.

```sym
Session s:
    Track as t:
        Starts: lonely
Clip lonely:
    Note
```
