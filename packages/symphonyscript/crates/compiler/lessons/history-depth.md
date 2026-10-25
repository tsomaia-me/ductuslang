# history-depth

`[N]` states the deepest offset a state reads, and a deeper offset is an error. (SPEC §6.6)

The runtime keeps N past values for the state.

```sym-error
input a = 1
state[1] s = a{2}
```

The read reaches 2 steps back, deeper than `[1]`.

```sym
input a = 1
state[2] s = a{2}
```
