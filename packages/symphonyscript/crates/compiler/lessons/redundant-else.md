# redundant-else

An `else:` after arms that cover every value is an error. (SPEC §5.4)

It could never be chosen.

```sym-error
state b(true)
let l = match b:
    true: 1
    false: 2
    else: 3
```

`true` and `false` cover every `bool`.

```sym
state b(true)
let l = match b:
    true: 1
    false: 2
```
