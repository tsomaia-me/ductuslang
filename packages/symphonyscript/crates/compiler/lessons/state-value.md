# state-value

A state holds no string and no node reference. (SPEC §6.6)

A state changes at runtime, and strings and node references exist only at compile time.

```sym-error
state s = "x"
```

The state holds a string.

```sym
state s = 1
```
