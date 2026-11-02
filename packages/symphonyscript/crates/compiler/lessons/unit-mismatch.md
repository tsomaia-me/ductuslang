# unit-mismatch

A unit value bound to an attribute of a different family, or of no family, is an error. (SPEC §3.6)

A unit says what a number measures, and an attribute measures one thing.

```sym-error
Scene s:
    Cue | fade=2bars
```

`fade` is measured in time, and `2bars` in beats.

```sym
Scene s:
    Cue | fade=2s
```
