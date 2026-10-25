# float-type-suffix

An integer type suffix on a float literal, as in `0.5i32`, is an error. (SPEC §2.6)

A float literal cannot be an integer type; a conversion says how to round it.

```sym-error
let a = 0.5i32
```

`0.5` is a float and `i32` an integer type.

```sym
let a = 0.5f32
```
