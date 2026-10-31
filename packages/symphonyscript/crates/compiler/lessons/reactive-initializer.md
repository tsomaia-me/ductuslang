# reactive-initializer

An input's initializer is a compile-time value. (SPEC §6.1)

It gives the input its value before the host writes one.

```sym-error
state a(1)
state b(a)
```

`b`'s initializer reads the input `a`.

```sym
let start = 1
state a(start)
state b(start)
```
