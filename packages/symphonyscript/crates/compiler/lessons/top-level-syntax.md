# top-level-syntax

A file holds top-level items only: inputs, `let`s, states, functions, patterns and top-level placements. (SPEC §2.1)

Every line at the left edge declares something, so a value standing alone there has nowhere to go.

```sym-error
5
```

A bare value at top level declares nothing.

```sym
let five = 5
```
