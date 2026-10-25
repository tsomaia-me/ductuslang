# for-variable-clash

A `for` variable equal to a visible name is an error. (SPEC §6.5)

The variable would hide that name in the loop's body.

```sym-error
Clip c:
    for sin in 0..2:
        Note
```

`sin` is a built-in.

```sym
Clip c:
    for step in 0..2:
        Note
```
