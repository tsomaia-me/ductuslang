# destination-type

A wire's destination is a node reference. (SPEC §8.2)

A wire connects two instances.

```sym-error
Scene s:
    Triggers: 1
```

`1` is no instance.

```sym
Scene s:
    Cue
    Triggers: s
```
