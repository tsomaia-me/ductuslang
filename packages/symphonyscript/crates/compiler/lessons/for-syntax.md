# for-syntax

A `for` is `for`, a name, `in`, an iterable, `:` and its body. (SPEC §7.9)

The name is the loop variable its body reads.

```sym-error
Clip c:
    for in 0..2:
        Note
```

The `for` has no variable.

```sym
Clip c:
    for i in 0..2:
        Note
```
