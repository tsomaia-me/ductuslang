# range-bound

The bounds of a `for` range are integers that fit `i64` (SPEC §4.6), and an upper bound that reads a cell and has no maximum by its form, or a lower bound that reads a cell and has no minimum, is an error (SPEC §7.9).

A range counts whole steps, and a `for` expression over a cell runs at most as many times as its bounds' forms allow, which is what its capacity is found from. `0..0.5` is an error too: `0.5` is no integer.

```sym-error
input n = 4

state total = for i in 0..n into sum from 0: sum + i

Clip c:
    Note/total
```

The bound `n` reads a cell, and its form gives it no maximum.

```sym
input n = 4

state total = for i in 0..min(n, 16) into sum from 0: sum + i

Clip c:
    Note/total
```

`0..min(n, 16)` runs at most 16 times.
