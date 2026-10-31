# reactive-string

A string expression that reads a cell is an error. (SPEC §3.3)

Strings exist only at compile time, and a cell changes at runtime.

```sym-error
state x(0.5)
Kit k:
    Sampler | file="{x}"
```

The interpolation reads the input `x`.

```sym
let x = 0.5
Kit k:
    Sampler | file="{x}"
```
