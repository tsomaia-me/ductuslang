# cell-capacity

`[N]` after `stream` or `channel` states a host-fed cell's pending capacity, and `[N]` on a `state` or on a derived cell is an error. (SPEC §6.1)

A host-fed `stream` or `channel` keeps every write since the last commit up to its capacity; a `state` keeps its last write alone, and a derived cell holds what its rule computes, so neither has pending writes to bound.

```sym-error
state[4] level(0)
```

`level` is a `state`, which keeps its last write alone.

```sym
stream[4] level(0)
```
