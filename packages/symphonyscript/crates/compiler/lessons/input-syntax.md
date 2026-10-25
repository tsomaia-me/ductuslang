# input-syntax

An input declaration is `input`, a name, an optional type, `=` and the initial value. (SPEC §6.1)

The host writes an input by its name.

```sym-error
input = 1
```

The input has no name.

```sym
input level = 1
```
