# misplaced-underscore

A single underscore may separate two digits, as in `1_000_000`; anywhere else it is an error. (SPEC §2.6)

The underscore only groups digits, so it stands between digits.

```sym-error
let a = 10_
```

The underscore ends the literal.

```sym
let a = 10
```
