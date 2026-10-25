# empty-bundle

A bundle has at least one member after expansion. (SPEC §7.8)

An empty bundle places nothing.

```sym-error
Clip c:
    [for i in 0..0: Note]
```

The `for` runs no iteration.

```sym
Clip c:
    [for i in 0..2: Note]
```
