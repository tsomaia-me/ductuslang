# no-default-attribute

`/expr` on a kind without a default attribute is an error. (SPEC §7.5)

`/expr` sets the default attribute, and the kind has none.

```sym-error
Plant p /1
```

`Plant` has no default attribute.

```sym
Plant p:
    Valve/1
```
