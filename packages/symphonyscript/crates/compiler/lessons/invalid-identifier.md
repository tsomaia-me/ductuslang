# invalid-identifier

An identifier holds at least one character other than `_` and `#`. (SPEC §2.4)

A name made only of `_` or `#` would read as punctuation.

```sym-error
let _ = 1
```

`_` alone is no name.

```sym
let unused = 1
```
