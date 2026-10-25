# index-type

An index is an integer. (SPEC §5.5)

It counts elements.

```sym-error
let a = [1, 2][true]
```

The index is a `bool`.

```sym
let a = [1, 2][1]
```
