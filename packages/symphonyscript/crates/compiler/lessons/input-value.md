# input-value

No input holds a string; strings exist only at compile time. (SPEC §3.3)

The host writes inputs at runtime, and runtime values are never strings.

```sym-error
input s = "text"
```

The input's initializer is a string.

```sym
let s = "text"
```
