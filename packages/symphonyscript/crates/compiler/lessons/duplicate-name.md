# duplicate-name

Top-level names share one namespace across the project, and a duplicate is an error naming both declarations. (SPEC §6.5)

A reference must name one declaration.

```sym-error
input a = 1
let a = 2
```

`a` is declared twice.

```sym
input a = 1
let b = 2
```
