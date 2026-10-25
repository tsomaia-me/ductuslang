# prefixed-suffix

A prefixed literal such as `0b101` takes no suffix, and a letter or digit right after its digits is an error. (SPEC §2.6)

A digit outside the prefix's base would otherwise read as part of the number.

```sym-error
let a = 0b12
```

`2` is no binary digit.

```sym
let a = 0b10
```
