# reserved-token

The tokens `==` and `!=` are reserved; equality is written `is` and `is not`. (SPEC §2.5)

One spelling for each comparison keeps programs readable to people who know no other language.

```sym-error
let a = 1 == 2
```

`==` is reserved.

```sym
let a = 1 is 2
```
