# plain-in-family

A plain number that meets a family value is read in that family's default unit, and where the result reaches no attribute of the family, as when a comparison makes it a `bool`, the plain value is an error. (SPEC §3.6)

Without an attribute of the family the plain number has no unit to be read in.

```sym-error
input d = 5ms
Clip c:
    Note when d > 1
```

`1` meets the time `d`, and the comparison reaches no time attribute.

```sym
input d = 5ms
Clip c:
    Note when d > 1ms
```
