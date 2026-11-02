# misplaced-to

`to` alone, a `to` path that ends at an instance or a wire, and `to` written outside a wire placement are errors. (SPEC §8.3)

`to` reads an attribute of a wire's destination.

```sym-error
let a = to.open
```

`to` stands outside a wire placement.

```sym
Plant p:
    Valve as v
    Controller:
        Drives | limit=to.open: v
```
