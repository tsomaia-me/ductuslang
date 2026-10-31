# history-offset

A history read's offset is a positive integer known at compile time. (SPEC §6.6)

The runtime keeps a fixed number of past values.

```sym-error
state a(1)
state s = a.past(0)
```

The offset is 0.

```sym
state a(1)
state s = a.past(1)
```
