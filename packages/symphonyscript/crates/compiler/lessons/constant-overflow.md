# constant-overflow

At compile time a fixed-type integer operation that saturates is an error. (SPEC §4.6)

A value known when the program compiles is computed exactly, so a saturated one is a mistake the compiler can see.

```sym-error
let x = 2147483647i32 + 1
Clip c:
    Note/x
```

The sum leaves the `i32` range.

```sym
let x = 2147483646i32 + 1
Clip c:
    Note/x
```
