# duplicate-local

Two parameters or two local `let`s with one name, or a local `let` named like a parameter, are an error. (SPEC §6.3)

Each name in a body names one value.

```sym-error
fn f(a, a): a
```

`a` names two parameters.

```sym
fn f(a, b): a
```
