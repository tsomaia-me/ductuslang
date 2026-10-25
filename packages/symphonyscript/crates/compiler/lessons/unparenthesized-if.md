# unparenthesized-if

An `if` that is an operand of another operator is parenthesized. (SPEC §5.1)

An `if` runs to the end of its `else`, so without parentheses it is unclear where it stops.

```sym-error
let a = 1 + if true then 1 else 2
```

The `if` is the right operand of `+`.

```sym
let a = 1 + (if true then 1 else 2)
```
