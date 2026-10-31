# state-value

A state holds no node reference. (SPEC §6.6)

A state changes at runtime, and node references exist only at compile time.

```sym-error
Clip c:
    Note
state s = c
```

The state holds a node reference.

```sym
state s = 1
```
