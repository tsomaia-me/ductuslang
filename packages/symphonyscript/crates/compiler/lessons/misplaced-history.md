# misplaced-history

History reads stand only in a state's initializer. (SPEC §6.6)

`let`s and functions stay pure: their values depend only on the present.

```sym-error
input a = 1
let b = a{1}
```

A `let` reads the history of `a`.

```sym
input a = 1
state b = a{1}
```
