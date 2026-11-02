# expansion-limit

A compilation runs at most 1,000,000 `for` iterations and pattern uses, the iterations of a compile-time `for` expression counting among them, and yields at most 1,000,000 instances, wires and cells of pattern uses, a pattern's declared cells counting once per use. (SPEC §4.6)

The limit keeps a mistaken bound from building an endless program.

```sym-error
Rig r:
    for i in 0..100001:
        Lamp Lamp Lamp Lamp Lamp Lamp Lamp Lamp Lamp Lamp
```

The loop yields over a million lamps.

```sym
Rig r:
    for i in 0..16:
        Lamp
```
