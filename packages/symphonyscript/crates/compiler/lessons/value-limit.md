# value-limit

No compile-time value holds more than 4,194,304 elements: an array counts its elements at every level, a string its characters and a join its whole length, and each value is checked when it is built, before anything walks it. Past the limit the compilation stops. (SPEC §4.6)

The limit keeps one value from taking the whole memory of the machine that compiles it.

```sym-error
fn twice(t, n): if n < 1 then t else twice(t + t, n - 1)
let name = twice("a", 26)
Track lead | label=name:
    Piano
```

Each call doubles the string, and the 23rd doubling builds 8,388,608 characters.

```sym
fn twice(t, n): if n < 1 then t else twice(t + t, n - 1)
let name = twice("a", 4)
Track lead | label=name:
    Piano
```
