# unbounded-state

A derived cell that holds a variable array has the least capacity for which its rule's capacity is at most its own; a cell with no such capacity is an error. (SPEC §6.6)

The runtime allocates every cell's buffer before it runs.

```sym-error
state reading(60)
state recent = recent.past(1, []) + [reading]
```

`recent` grows by one element at every step.

```sym
state reading(60)
state recent = (recent.past(1, []) + [reading])[^8..]
```
