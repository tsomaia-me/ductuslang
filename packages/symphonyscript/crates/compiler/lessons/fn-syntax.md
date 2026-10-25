# fn-syntax

A function is `fn`, a name, its parameters in parentheses, an optional bound, `:` and its body. (SPEC §6.3)

`:` starts every body in the language, a function's included.

```sym-error
fn double(x) = x * 2
```

The body follows `=` instead of `:`.

```sym
fn double(x): x * 2
```
