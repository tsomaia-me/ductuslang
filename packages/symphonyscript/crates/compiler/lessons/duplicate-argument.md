# duplicate-argument

Supplying a pattern parameter twice is an error. (SPEC §6.4)

A parameter binds one value.

```sym-error
pattern P(default root):
    Note/root
Clip c:
    P/1 | root=2
```

`root` is supplied by `/1` and by `root=2`.

```sym
pattern P(default root):
    Note/root
Clip c:
    P/1
```
