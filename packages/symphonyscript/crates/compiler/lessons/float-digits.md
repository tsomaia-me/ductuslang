# float-digits

A float has digits on both sides of the point: `0.5`, not `.5`. (SPEC §2.6)

A point with no digit before it is easy to miss.

```sym-error
let a = .5
```

The literal starts with the point.

```sym
let a = 0.5
```
