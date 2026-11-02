# type-mismatch

Every value has the type its position expects: a number where a number is expected, a node reference where one is expected, and so on. (SPEC §4.2)

Each attribute, annotation and operand states one type, and a value of another type has no meaning there.

```sym-error
Plant p:
    Valve | open=p
```

`open` is a number, and `p` is a node reference.

```sym
Plant p:
    Valve | open=0.8
```
