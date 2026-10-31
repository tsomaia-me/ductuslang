# to-candidates

With a destination chosen at runtime, every candidate has the `to` path, ending at an attribute of the same type and unit family. (SPEC §8.3)

`to` reads whichever candidate the wire chooses, so every candidate gives one type.

```sym-error bounded
state i(0)
A one
B two
Root r:
    Item
    Link: r
    Pick | level=to.x: [one, two][i]
```

`x` is an `i32` on `A` and an `f32` on `B`.

```sym bounded
state i(0)
A one
A two
Root r:
    Item
    Link: r
    Pick | level=to.x: [one, two][i]
```
