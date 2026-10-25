# inconsistent-uses

A flexible value with an integer use and a float use has the same value at both; otherwise it is an error naming both uses. (SPEC §4.1)

One value read two ways must mean one number.

```sym-error
let q = 2 ** -1
Synth s:
    Osc/q
Clip c:
    Note/q
```

`q` is 0.5 as a frequency and 0 as a pitch.

```sym
let q = 2 ** 1
Synth s:
    Osc/q
Clip c:
    Note/q
```
