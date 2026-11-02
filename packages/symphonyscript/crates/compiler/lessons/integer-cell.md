# integer-cell

An integer host-fed cell whose every use expects a float draws a warning. (SPEC §4.4)

Its host cannot write a fraction, while every use reads a float; `1.0` in place of `1` makes it a float cell.

```sym-error
state n(1)
Plant p:
    Valve | open=n
```

`n` is an integer cell, and its one use reads a float.

```sym
state n(1.0)
Plant p:
    Valve | open=n
```
