# type-syntax

A type is a scalar type or an enum, followed by lengths in `[N]` and at most one capacity `[..N]`, the last. (SPEC §3.2)

A type's lengths are fixed when the program compiles, so they are digits.

```sym-error
state a([1]): i32[x]
```

`x` is no length.

```sym
state a([1]): i32[1]
```
