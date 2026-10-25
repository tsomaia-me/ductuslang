# conflicting-units

A plain number read through a family value is read in the default unit of the attribute it reaches, and a family value that reaches two attributes of the family with different default units is an error. (SPEC §3.6)

The plain part would mean two different amounts at the two attributes.

```sym-error bars
let d = 2bars + 1
Bar b | bar_len=d beat_len=d
```

`1` would be one bar at `bar_len` and one beat at `beat_len`.

```sym bars
let d = 2bars + 1bars
Bar b | bar_len=d beat_len=d
```
