# for-syntax

A `for` is `for`, one name or two separated by `,`, each with an optional annotation, `in`, an iterable, `:` and its body. (SPEC §7.9)

The names are the loop's variables its body reads.

```sym-error
Clip c:
    for in 0..2:
        Note
```

The `for` has no name.

```sym
Clip c:
    for i in 0..2:
        Note
```
