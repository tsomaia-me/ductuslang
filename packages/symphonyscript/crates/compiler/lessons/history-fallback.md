# history-fallback

A history read states its fallback, of the cell's type, except for an input, whose fallback is its initial value. (SPEC §6.6)

Before a state has taken k steps there is no past value to read.

```sym-error
state s = s.past(1) + 1
```

The read of `s` states no fallback.

```sym
state s = s.past(1, 0) + 1
```
