# lambda-syntax

A lambda is a parameter name, `:` and its body, or its parameters in parentheses, each a name with an optional annotation, an optional result `-> T`, `:` and its body. (SPEC §6.7)

A lambda is a function written in place, so its parameters and its body stand as a function's do.

```sym-error
let add = (a b): a + b
```

The parameters are not separated by a comma.

```sym unbuilt
let add = (a, b): a + b
```
