# unknown-parameter

A use's attribute clause supplies the pattern's parameters, and a name that is no parameter is an error. (SPEC §6.4)

The clause binds parameters by their names.

```sym-error
pattern P(level = 1):
    Lamp/level
Rig r:
    P | brightness=1
```

`P` has no parameter `brightness`.

```sym
pattern P(level = 1):
    Lamp/level
Rig r:
    P | level=0.5
```
