# not-an-array

Only an array is indexed, sliced or measured with `len`. (SPEC §5.5)

A single value has no elements.

```sym-error
let a = 1[0]
```

`1` is a single value.

```sym
let a = [1][0]
```
