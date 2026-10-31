# map-syntax

A map literal is written in braces and lists its entries separated by commas, each a key, `:` and a value; `{}` is the empty map. (SPEC §3.10)

A map holds keys with their values, so each entry states both.

```sym-error
let ports = {"http" 80}
```

The entry has no `:` between its key and its value.

```sym unbuilt
let ports = {"http": 80}
```
