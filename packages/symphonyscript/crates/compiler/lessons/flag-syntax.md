# flag-syntax

A flag run ends before any identifier character. (SPEC §7.4)

A letter after a flag would read as part of a name.

```sym-error
Clip c:
    Note'x
```

`x` follows the flag `'`.

```sym
Clip c:
    Note'
```
