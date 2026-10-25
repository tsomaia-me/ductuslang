# type-mismatch

Every value has the type its position expects: a number where a number is expected, a node reference where one is expected, and so on. (SPEC §4.2)

Each attribute, annotation and operand states one type, and a value of another type has no meaning there.

```sym-error
Clip c:
    Note | velocity=c
```

`velocity` is a number, and `c` is a node reference.

```sym
Clip c:
    Note | velocity=0.8
```
