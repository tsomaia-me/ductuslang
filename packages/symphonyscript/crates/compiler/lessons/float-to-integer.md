# float-to-integer

A float input used where an integer is expected is an error. (SPEC §4.4)

An integer position would need to round the input, and the program states no rounding.

```sym-error
input cutoff = 0.5
Clip c:
    Note/cutoff
```

The pitch is an integer, and `cutoff` is a float input.

```sym
input cutoff = 0.5
Clip c:
    Note/(i32(cutoff * 100))
```
