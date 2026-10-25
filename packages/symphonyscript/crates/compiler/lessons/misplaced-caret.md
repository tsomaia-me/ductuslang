# misplaced-caret

`^` stands only at the start of an index or a slice bound, where `^k` counts from the end; anywhere else it is an error. (SPEC §2.5)

`^` has one meaning, counting from an array's end, and no operator uses it.

```sym-error
let a = 2 ^ 3
```

`^` stands between two numbers; powers are written `**`.

```sym
let a = 2 ** 3
let xs = [10, 20, 30]
let last = xs[^1]
```
