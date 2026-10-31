# history-fallback

A derived cell whose rule reads its own past, directly or through other cells' pasts, has a value at load when it or a cell on that cycle has an init, or a read on the cycle a fallback; with neither the cycle is an error. (SPEC §6.6)

At load the history holds no value yet, and the init or the fallback is the value the read takes then.

```sym-error
state s = s.past(1) + 1
```

`s` reads its own past, and neither `s` has an init nor the read a fallback.

```sym
state s(0) = s.past(1) + 1
```
