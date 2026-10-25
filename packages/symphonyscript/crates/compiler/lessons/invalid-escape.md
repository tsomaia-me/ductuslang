# invalid-escape

A backslash in a string starts one of the escapes `\"`, `\\`, `\n`, `\t` and `\{`; any other is an error. (SPEC §2.6)

An unknown escape would leave the reader guessing which character it means.

```sym-error
let s = "a\q"
```

`\q` is no escape.

```sym
let s = "a\tq"
```
