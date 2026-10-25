# misplaced-to

`to` alone, a `to` path that ends at an instance or a wire, and `to` written outside a wire placement are errors. (SPEC §8.3)

`to` reads an attribute of a wire's destination.

```sym-error
let a = to.gain
```

`to` stands outside a wire placement.

```sym
Clip a:
    Note
Clip c:
    Note
    Plays | gain=to.gain: a
```
