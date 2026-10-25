# variant-name

A declared name equal to a variant of a vocabulary enum is an error. (SPEC §6.5)

A bare variant such as `Saw` reads as the variant where an enum is expected.

```sym-error
let Saw = 1
```

`Saw` is a variant of `Wave`.

```sym
let saw_level = 1
```
