# inconsistent-shapes

A value that holds a recursive call's array result has one shape at all its uses; different shapes at two uses are an error naming both. (SPEC §4.6)

The result's length can depend on the type its use computes it in.

```sym-error
fn r(x, n): if n < 1 then (if x + 0.2 is 0.3 then [x] else [x, x]) else r(x, n - 1)
let v = r(0.1, 3)
Clip c | gain=(v[0])
Osc o | freq=(v[0])
```

In `f32` the call yields one element and in `f64` two.

```sym
fn r(x, n): if n < 1 then [x, x] else r(x, n - 1)
let v = r(0.1, 3)
Clip c | gain=(v[0])
Osc o | freq=(v[0])
```
