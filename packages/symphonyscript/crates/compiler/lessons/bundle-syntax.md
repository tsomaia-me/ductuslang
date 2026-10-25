# bundle-syntax

A bundle is `[`, its members or a `for`, and `]`. (SPEC §7.8)

The brackets mark which placements share the bundle.

```sym-error
Clip c:
    [Note
```

The bundle never closes.

```sym
Clip c:
    [Note Note]
```
