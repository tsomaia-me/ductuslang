# missing-argument

Omitting a parameter that has no default is an error. (SPEC §6.4)

The parameter would have no value.

```sym-error
pattern P(root):
    Note/root
Clip c:
    P
```

`root` has no default and no argument.

```sym
pattern P(root):
    Note/root
Clip c:
    P | root=60
```
