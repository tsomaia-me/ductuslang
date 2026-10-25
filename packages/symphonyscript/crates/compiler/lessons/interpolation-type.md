# interpolation-type

`{expr}` in a string interpolates a compile-time value: a number, a `bool`, an enum variant, a string or a unit value. (SPEC §5.9)

The interpolated value becomes text when the program compiles.

```sym-error
Kit k:
    Sampler | file="{k}"
```

`k` is a node reference.

```sym
let take = 3
Kit k:
    Sampler | file="take{take}.wav"
```
