# duplicate-attribute

Setting an attribute twice, by any combination of entries, flags and `/expr`, is an error. (SPEC §7.6)

An attribute holds one value.

```sym-error
Plant p:
    Valve/0.5 | open=0.6
```

`/0.5` and `open=0.6` both set the opening.

```sym
Plant p:
    Valve/0.6
```
