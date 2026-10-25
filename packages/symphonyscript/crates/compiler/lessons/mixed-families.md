# mixed-families

`+`, `-`, `%`, comparisons, `min`, `max`, `clamp`, the first two arguments of `lerp`, array literals and the arms of `if` and `match` take operands of one family. (SPEC §3.6)

Adding seconds to bars has no meaning.

```sym-error
Synth s:
    Osc | freq=2s + 1bars
```

The sum adds time to musical time.

```sym
Clip c:
    Note | length=2bars + 1bars
```
