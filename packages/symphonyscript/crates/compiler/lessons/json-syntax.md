# json-syntax

A document literal is the keyword `json` followed by JSON text: an object, an array, a string with JSON's escapes, a number without a prefix, a separator or a suffix, `true`, `false`, `null`, a name, a path or a parenthesized expression. (SPEC §3.11)

The text after `json` is JSON's own, so a document is pasted into a program unchanged.

```sym-error
let doc = json {"count": 0x10}
```

`0x10` is no JSON number.

```sym unbuilt
let doc = json {"count": 16}
```
