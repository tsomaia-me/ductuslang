# unterminated-string

A string occupies one line and ends with `"` on it. (SPEC §2.6)

A string that ran on would swallow the lines after it.

```sym-error
let s = "abc
```

The string has no closing quote.

```sym
let s = "abc"
```
