# duplicate-attribute

Setting an attribute twice, by any combination of entries, flags and `/expr`, is an error. (SPEC §7.6)

An attribute holds one value.

```sym-error
Clip c:
    Note/60 | pitch=61
```

`/60` and `pitch=61` both set the pitch.

```sym
Clip c:
    Note/61
```
