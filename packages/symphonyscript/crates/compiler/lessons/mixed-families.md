# mixed-families

`+`, `-`, `%`, comparisons, `min`, `max`, `clamp`, the first two arguments of `lerp`, array literals and the arms of `if` and `match` take operands of one family. (SPEC §3.6)

Adding seconds to bars has no meaning.

```sym-error
Scene s | hold=2s + 1bars
```

The sum adds a time to a count of beats.

```sym
Scene s | hold=2s + 500ms
```
