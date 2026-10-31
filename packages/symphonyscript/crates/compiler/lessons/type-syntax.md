# type-syntax

A type is a scalar type, `str`, `json`, a map type `{K: V}[..N]`, a name or `none`, with its type arguments, then lengths in `[N]` and capacities in `[..N]`, each a size, and an optional `?`; alternatives join with `|`, `( )` groups a type, and `(T1, T2) -> R` is a function type. (SPEC §3.2)

A type is read where the program compiles, so every bracket holds a size and every part stands in its place.

```sym-error
state a([1]): i32[]
```

The brackets hold no size.

```sym
state a([1]): i32[1]
```
