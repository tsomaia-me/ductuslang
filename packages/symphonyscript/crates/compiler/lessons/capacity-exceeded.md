# capacity-exceeded

A value bound to a variable-array attribute or annotation holds at most its capacity. (SPEC §3.7)

The runtime holds every variable array in a buffer of its capacity.

```sym-error natives
input held: i32[..10] = []
Clip c | sequence=held:
    Note
```

`held` holds up to 10 elements, and `sequence` up to 8.

```sym natives
input held: i32[..10] = []
Clip c | sequence=(held[^8..]):
    Note
```

A string where a `u8` array is expected holds its bytes, and a choice of strings the bytes of its longest one, so a string past the capacity takes a shorter string or a larger capacity: `"GET"` holds 3 bytes, so `word` needs a capacity of at least 3.

```sym natives
input loud = false
let word: u8[..3] = if loud then "GET" else "AB"

Clip c:
    Note/(checksum(word))
```
