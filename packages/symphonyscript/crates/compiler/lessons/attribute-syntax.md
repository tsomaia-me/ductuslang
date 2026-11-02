# attribute-syntax

An attribute entry is `name=value`, `name` or `!name`. (SPEC §7.6)

Each entry sets the attribute it names.

```sym-error
Rig r:
    Lamp | =1
```

The entry names no attribute.

```sym
Rig r:
    Lamp | level=1
```
