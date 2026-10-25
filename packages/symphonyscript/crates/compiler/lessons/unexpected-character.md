# unexpected-character

A source holds only the characters its tokens, strings and comments use; any other character is an error. (SPEC §2.1)

A character that starts no token has no meaning, so the compiler names it rather than guess.

```sym-error
let a = 1 & 2
```

`&` starts no token; logic is written with `and`.

```sym
let a = true and false
```
