# no-default-parameter

`/expr` on a pattern without a `default` parameter is an error. (SPEC §6.4)

`/expr` supplies the `default` parameter, and there is none.

```sym-error
pattern P(root):
    Note/root
Clip c:
    P/1 | root=2
```

`P` marks no parameter `default`.

```sym
pattern P(default root):
    Note/root
Clip c:
    P/1
```
