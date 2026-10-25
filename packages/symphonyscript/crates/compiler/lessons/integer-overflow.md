# integer-overflow

A flexible integer whose magnitude reaches 2¹²⁷ is an error. (SPEC §4.6)

Flexible integers compute exactly below 2¹²⁷.

```sym-error
Clip c:
    Note/(2 ** 200)
```

`2 ** 200` reaches 2¹²⁷.

```sym
Clip c:
    Note/(2 ** 6)
```
