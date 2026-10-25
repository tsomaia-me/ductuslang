# operand-type

Each operator takes operands of the types it allows: `div` integers, `and` and `or` `bool`s, and so on. (SPEC §5.2)

An operator's result is defined only for the operand types it names.

```sym-error
fn f(x): if x < 1 then x div 2 else f(x - 1)
Clip c:
    Note/(f(0.5))
```

The call computes `div` on a float.

```sym
fn f(x): if x < 1 then x div 2 else f(x - 1)
Clip c:
    Note/(f(5))
```
