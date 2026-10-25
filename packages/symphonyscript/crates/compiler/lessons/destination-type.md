# destination-type

A wire's destination is a node reference. (SPEC §8.2)

A wire connects two instances.

```sym-error
Clip c:
    Plays: 1
```

`1` is no instance.

```sym
Clip c:
    Note
    Plays: c
```
