# conversion-type

`u8`, `i32`, `i64`, `f32` and `f64` convert numbers; converting a `bool` or an enum value is an error. (SPEC §5.8)

A `bool` or a variant has no number to convert.

```sym-error
let a = f32(true)
Clip c:
    Note/a
```

`true` is a `bool`.

```sym
let a = f32(1)
```
