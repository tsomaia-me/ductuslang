# type-syntax

A type is a scalar type or an enum, followed by lengths in `[N]` and at most one capacity `[..N]`, the last. (SPEC §3.2)

A type's lengths are fixed when the program compiles, so they are digits.

```sym-error
input a: i32[x] = [1]
```

`x` is no length.

```sym
input a: i32[1] = [1]
```
