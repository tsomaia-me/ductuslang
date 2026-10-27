# named-builtin-arguments

Built-ins and native functions take positional arguments only. (SPEC §5.7)

Their parameters have no names in the source.

```sym-error
let a = sin(x=1)
Clip c:
    Note/a
```

The argument of `sin` is named.

```sym
let a = sin(1)
```
