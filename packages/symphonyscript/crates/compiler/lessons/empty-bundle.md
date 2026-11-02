# empty-bundle

A bundle has at least one member after expansion. (SPEC §7.8)

An empty bundle places nothing.

```sym-error
Plant p:
    [for i in 0..0: Valve]
```

The `for` runs no iteration.

```sym
Plant p:
    [for i in 0..2: Valve]
```
