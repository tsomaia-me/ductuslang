# reserved-word

The words `from`, `recurrent` and `string` are reserved and are never names. (SPEC §2.5)

They are kept free for the language to use.

```sym-error
let from = 1
```

`from` is reserved.

```sym
let start = 1
```
