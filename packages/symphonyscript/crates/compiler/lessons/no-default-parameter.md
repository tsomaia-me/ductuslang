# no-default-parameter

`/expr` on a pattern without a `default` parameter is an error. (SPEC §6.4)

`/expr` supplies the `default` parameter, and there is none.

```sym-error
pattern P(level):
    Lamp/level
Rig r:
    P/1 | level=0.5
```

`P` marks no parameter `default`.

```sym
pattern P(default level):
    Lamp/level
Rig r:
    P/1
```
