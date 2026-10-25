# zero-length

A length or a capacity in a type is at least 1. (SPEC §3.2)

A type of no elements could hold nothing; `[]` is the empty array.

```sym-error
input a: i32[0] = [1]
```

The type has a length of 0.

```sym
input a: i32[1] = [1]
```
