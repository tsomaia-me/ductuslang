# prelude-item

The prelude holds `let`, `fn` and `pattern` declarations only. (SPEC §10.1)

The prelude is shared by every program of a vocabulary, so it declares no cells and no instances.

```sym-error prelude
input a = 1
```

The prelude declares an input.

```sym prelude
let a = 1
```
