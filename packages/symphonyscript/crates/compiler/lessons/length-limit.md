# length-limit

An array's length and capacity are below 2³¹. (SPEC §3.2)

Lengths, indexes and capacities share one 32-bit range at runtime.

```sym-error
state a([]): i32[..2147483648]
```

The capacity is 2³¹.

```sym
state a([]): i32[..2147483647]
```
