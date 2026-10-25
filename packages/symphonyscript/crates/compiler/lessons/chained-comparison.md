# chained-comparison

Comparisons do not chain: `a < b < c` is an error. (SPEC §5.1)

A chain could mean two comparisons or a comparison of a `bool`, and the language picks neither.

```sym-error
let a = 1 < 2 < 3
```

Two comparisons are chained.

```sym
let a = 1 < 2 and 2 < 3
```
