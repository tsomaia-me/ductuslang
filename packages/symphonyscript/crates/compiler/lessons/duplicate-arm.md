# duplicate-arm

A label that repeats an earlier one is an error. (SPEC §5.4)

The first matching arm wins, so a repeated arm never runs.

```sym-error
input w = Wave::Saw
let l = match w:
    Sine: 1
    Sine: 2
    else: 3
```

`Sine` labels two arms.

```sym
input w = Wave::Saw
let l = match w:
    Sine: 1
    Saw: 2
    else: 3
```
