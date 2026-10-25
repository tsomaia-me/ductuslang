# reactive-initializer

An input's initializer is a compile-time value. (SPEC §6.1)

It gives the input its value before the host writes one.

```sym-error
input a = 1
input b = a
```

`b`'s initializer reads the input `a`.

```sym
let start = 1
input a = start
input b = start
```
