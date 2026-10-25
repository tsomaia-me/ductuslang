# match-scrutinee

A `match` selects by an enum, an integer or a `bool`; a scrutinee of another type is an error. (SPEC §5.4)

A `match` chooses among labels, and enums, integers and `bool`s are the values labels name.

```sym-error
let l = match 1.5:
    1: 2
    else: 1
```

The scrutinee is a float.

```sym
let l = match 1:
    1: 2
    else: 1
```
