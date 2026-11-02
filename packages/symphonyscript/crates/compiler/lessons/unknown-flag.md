# unknown-flag

A flag is a character the vocabulary assigns to an attribute of the kind. (SPEC §7.4)

A flag sets its attribute, so an unassigned flag sets nothing.

```sym-error
Rig r:
    Lamp!
```

`Lamp` assigns no attribute to `!`.

```sym
Rig r:
    Lamp*
```
