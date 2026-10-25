# unit-operation

Dividing a plain number by a unit value, multiplying two unit values, `**` on a unit value and a unit value as a range bound are errors. (SPEC §3.6)

These operations would yield units the language has no family for.

```sym-error
Clip c:
    Note | length=2bars * 2bars
```

Bars times bars is no length.

```sym
Clip c:
    Note | length=2bars * 2
```
