# mixed-arguments

A call's arguments are all positional or all named. (SPEC §5.7)

Mixing the two forms leaves unclear which parameter a positional argument binds.

```sym-error
fn f(a, b): a
let a = f(1, b=2)
```

One argument is positional and the other named.

```sym
fn f(a, b): a
let a = f(a=1, b=2)
```
