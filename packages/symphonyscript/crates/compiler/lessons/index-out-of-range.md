# index-out-of-range

An index known at compile time stands inside a fixed array, and an index or a slice bound known at compile time fits `i64`. (SPEC §5.5)

The compiler sees the index and the length, so an index outside them is a mistake.

```sym-error
Clip c:
    Note/([1, 2, 3][5])
```

The array holds three elements.

```sym
Clip c:
    Note/([1, 2, 3][2])
```
