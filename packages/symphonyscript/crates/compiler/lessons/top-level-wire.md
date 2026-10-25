# top-level-wire

A wire is placed inside its source's body; a wire kind at top level is an error. (SPEC §7.1)

A wire's source is the instance whose body holds it.

```sym-error
Plays p: c
```

The wire stands at top level.

```sym
Clip c:
    Note
    Plays: c
```
