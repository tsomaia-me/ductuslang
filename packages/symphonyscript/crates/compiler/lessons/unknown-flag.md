# unknown-flag

A flag is a character the vocabulary assigns to an attribute of the kind. (SPEC §7.4)

A flag sets its attribute, so an unassigned flag sets nothing.

```sym-error
Clip c:
    Note!
```

`Note` assigns no attribute to `!`.

```sym
Clip c:
    Note'
```
