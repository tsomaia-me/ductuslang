# unknown-parameter

A use's attribute clause supplies the pattern's parameters, and a name that is no parameter is an error. (SPEC §6.4)

The clause binds parameters by their names.

```sym-error
pattern P(root = 1):
    Note/root
Clip c:
    P | octave=1
```

`P` has no parameter `octave`.

```sym
pattern P(root = 1):
    Note/root
Clip c:
    P | root=2
```
