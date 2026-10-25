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
