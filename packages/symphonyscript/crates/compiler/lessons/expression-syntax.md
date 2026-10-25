# expression-syntax

An expression follows the grammar of §5.1: every operator has its operands, and every bracket closes. (SPEC §5.1)

A half-written expression has no value.

```sym-error
let a = (1 +)
```

`+` has no right operand.

```sym
let a = (1 + 2)
```
