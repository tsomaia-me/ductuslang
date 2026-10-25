# array-operand

`+` of two arrays joins them; every other operator takes single values, and `+` of an array and a single value is an error. (SPEC §5.5)

An operator on whole arrays would leave unclear which elements it combines.

```sym-error
let a = [1, 2] + 3
```

`+` meets an array and a single value.

```sym
let a = [1, 2] + [3]
```
