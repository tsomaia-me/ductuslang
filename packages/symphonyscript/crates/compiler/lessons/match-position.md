# match-position

`match` is the whole value of a `let`, of a function body or of a wire destination, never an operand. (SPEC §5.4)

Its arms take whole lines, which an operand cannot hold.

```sym-error
let a = 1 + match 1:
    else: 2
```

The `match` is an operand of `+`.

```sym
let m = match 1:
    1: 3
    else: 2
let a = 1 + m
```
