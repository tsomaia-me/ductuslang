# unit-operation

Dividing a plain number by a unit value, multiplying two unit values, `**` on a unit value, and a unit value as a range bound, an index, a slice bound or a `^` offset are errors. (SPEC §3.6)

These operations would yield units the language has no family for.

```sym-error
Scene s:
    Cue | fade=2s * 2s
```

Seconds times seconds is no duration.

```sym
Scene s:
    Cue | fade=2s * 2
```
