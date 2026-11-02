# missing-argument

Omitting a parameter that has no default is an error. (SPEC §6.4)

The parameter would have no value.

```sym-error
pattern P(amount):
    Valve/amount
Plant p:
    P
```

`amount` has no default and no argument.

```sym
pattern P(amount):
    Valve/amount
Plant p:
    P | amount=0.5
```
