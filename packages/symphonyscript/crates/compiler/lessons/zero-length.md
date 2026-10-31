# zero-length

A length or a capacity in a type is at least 1. (SPEC §3.2)

A type of no elements could hold nothing; `[]` is the empty array.

```sym-error
state a([1]): i32[0]
```

The type has a length of 0.

```sym
state a([1]): i32[1]
```
