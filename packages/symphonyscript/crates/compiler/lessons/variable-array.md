# variable-array

A variable array where a fixed array is expected is an error. (SPEC §3.7)

A fixed array has a length known when the program compiles, and a variable array's length changes at runtime.

```sym-error
state a([]): i32[..4]
let b: i32[2] = a
```

`b` is annotated as a fixed array of two, and `a` is a variable array.

```sym
state a([]): i32[..4]
let b: i32[..4] = a
```
