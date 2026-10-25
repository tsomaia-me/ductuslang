# expansion-limit

A compilation runs at most 1,000,000 `for` iterations and pattern uses and yields at most 1,000,000 instances and wires. (SPEC §4.6)

The limit keeps a mistaken bound from building an endless program.

```sym-error
Clip c:
    for i in 0..100001:
        Note Note Note Note Note Note Note Note Note Note
```

The loop yields over a million notes.

```sym
Clip c:
    for i in 0..16:
        Note
```
