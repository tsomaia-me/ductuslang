# match-incomplete

A `match` on an enum or a `bool` covers every value or ends with `else:`, and a `match` on an integer ends with `else:`. (SPEC §5.4)

Every value of the scrutinee needs an arm to yield.

```sym-error
state m(Mode::Manual)
state l = match m:
    Auto: 1
```

`Manual` and `Off` have no arm.

```sym
state m(Mode::Manual)
state l = match m:
    Auto: 1
    else: 2
```
