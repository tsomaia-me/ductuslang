# integer-input

An integer input whose every use expects a float draws a warning. (SPEC §4.4)

Its host cannot write a fraction, while every use reads a float; `4.0` in place of `4` makes it a float input.

```sym-error
input n = 4
Clip c:
    Note | velocity=n
```

`n` is an integer input, and its one use reads a float.

```sym
input n = 4.0
Clip c:
    Note | velocity=n
```
