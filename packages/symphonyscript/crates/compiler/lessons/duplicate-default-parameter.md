# duplicate-default-parameter

At most one pattern parameter is marked `default`. (SPEC §6.4)

`/expr` on a use supplies the `default` parameter, so there is one.

```sym-error
pattern P(default a, default b):
    Note/a
```

Two parameters are marked `default`.

```sym
pattern P(default a, b = 1):
    Note/a
```
