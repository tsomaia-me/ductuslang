# integer-too-large

An integer literal whose magnitude is 2¹²⁷ or more is an error. (SPEC §2.6)

The compiler computes integers exactly below 2¹²⁷ and nowhere above.

```sym-error
let a = 170141183460469231731687303715884105728
```

The literal is 2¹²⁷.

```sym
let a = 170141183460469231731687303715884105727
```
