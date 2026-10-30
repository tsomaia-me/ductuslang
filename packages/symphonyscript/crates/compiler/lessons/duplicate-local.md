# duplicate-local

Two parameters or two local `let`s with one name, or a local `let` named like a parameter, are an error (SPEC §6.3), and so are two equal names among one loop head's names, a `for` expression's accumulator and its block form's local `let`s (SPEC §6.5).

Each name in a body names one value.

```sym-error
fn f(a, a): a
```

`a` names two parameters.

```sym
fn f(a, b): a
```
