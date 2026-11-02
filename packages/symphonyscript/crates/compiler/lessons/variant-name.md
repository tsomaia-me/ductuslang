# variant-name

A declared name equal to a variant of a vocabulary enum is an error. (SPEC §6.5)

A bare variant such as `Manual` reads as the variant where an enum is expected.

```sym-error
let Manual = 1
```

`Manual` is a variant of `Mode`.

```sym
let manual_level = 1
```
