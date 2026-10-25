# match-incomplete

A `match` on an enum or a `bool` covers every value or ends with `else:`, and a `match` on an integer ends with `else:`. (SPEC §5.4)

Every value of the scrutinee needs an arm to yield.

```sym-error
input w = Wave::Saw
let l = match w:
    Sine: 1
```

`Saw` and `Square` have no arm.

```sym
input w = Wave::Saw
let l = match w:
    Sine: 1
    else: 2
```
