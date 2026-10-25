# conversion-syntax

A conversion writes its type and its value in parentheses: `i32(x)`. (SPEC §5.8)

The parentheses mark where the converted value ends.

```sym-error
let a = i32 1
```

The value has no parentheses.

```sym
let a = i32(1)
```
