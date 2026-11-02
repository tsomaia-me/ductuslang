# placement-syntax

A top-level placement is a kind and a name, then optional flags, a default value, a gate, an attribute clause and a body. (SPEC §7.1)

A top-level instance is reached by its name.

```sym-error
Plant
```

The top-level `Plant` has no name.

```sym
Plant boiler
```
