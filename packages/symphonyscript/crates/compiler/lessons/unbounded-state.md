# unbounded-state

A state that holds a variable array has the least capacity that holds its initializer; a state with no such capacity is an error. (SPEC §6.6)

The runtime allocates every state's buffer before it runs.

```sym-error
input note = 60
state recent = recent{1, []} + [note]
```

`recent` grows by one element at every step.

```sym
input note = 60
state recent = (recent{1, []} + [note])[^8..]
```
