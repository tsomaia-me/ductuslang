# index-out-of-range

An index known at compile time stands inside a fixed array, and an index or a slice bound known at compile time fits `i64`. (SPEC §5.5)

The compiler sees the index and the length, so an index outside them is a mistake.

```sym-error
Clip c:
    Note/([1, 2, 3][5])
```

The array holds three elements: keep the index inside the array, from 0 to its length minus one.

```sym
Clip c:
    Note/([1, 2, 3][2])
```

An empty array holds no element to read: give it elements, or read the element only where its `size` is above 0.

```sym natives
Clip a:
    Note/(if b.sequence.size > 0 then b.sequence[0] else 60)
Clip b:
    Note
```
