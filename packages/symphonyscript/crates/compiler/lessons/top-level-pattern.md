# top-level-pattern

Only a pattern that stands in for a single placement is used at top level. (SPEC §6.4)

A top-level use names one instance.

```sym-error
pattern P():
    Note
    Note
P top
```

`P` expands to two placements.

```sym
pattern P():
    Clip:
        Note
        Note
P top
```
