# named-in-for

Placements in a `for` body are anonymous; `as` there is an error. (SPEC §7.9)

Every iteration would give its placement the same name.

```sym-error
Clip c:
    for i in 0..2:
        Note as n
```

The placement in the loop is named.

```sym
Clip c:
    for i in 0..2:
        Note
```
