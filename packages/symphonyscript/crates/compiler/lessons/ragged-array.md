# ragged-array

An array's elements are all single values or all arrays of one length. (SPEC §3.2)

An array has one element type, so its rows share one shape.

```sym-error
let a = [1, [2]]
Clip c:
    Note/a
```

One element is a number and the other an array.

```sym
let a = [[1], [2]]
```
