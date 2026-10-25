# unknown-attribute

An attribute that is read or set is one the kind declares. (SPEC §7.6)

The vocabulary states each kind's attributes, so any other name is a mistake.

```sym-error
Clip c | gain=c.nothing:
    Note
```

`Clip` declares no attribute `nothing`.

```sym
Clip c | gain=c.level:
    Note
```
