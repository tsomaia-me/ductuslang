# unparenthesized-for

A `for` expression that is not the whole value of a `let`, a local `let`, a derived cell's rule or a function body stands in parentheses. (SPEC §5.10)

A `for` expression runs to the end of its body, so without parentheses it is unclear where it stops.

```sym-error
state held([]): i32[..10]
state total = 1 + for x in held into sum from 0: sum + x
```

The `for` expression is the right operand of `+`; in parentheses it is one value.

```sym
state held([]): i32[..10]
state total = 1 + (for x in held into sum from 0: sum + x)
```
