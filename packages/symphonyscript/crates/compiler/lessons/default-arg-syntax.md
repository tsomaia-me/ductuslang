# default-arg-syntax

An unparenthesized `/expr` is a single atom; anything else is parenthesized, as in `Note/(base + 7)`. (SPEC §7.5)

The value ends where the placement's next part begins.

```sym-error
Clip c:
    Note/a[0]
```

The index makes the value more than an atom.

```sym
let a = [60]
Clip c:
    Note/(a[0])
```
