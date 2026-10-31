# record-syntax

A `type` declaration is `type`, a name, optional bracket parameters, and either `:` with one field per indented line, each a name, `:` and a type, or `=` and a type; a record literal is its type's name and its fields in braces, each a name, `:` and a value, separated by commas. (SPEC §3.8)

A record is a value with named fields, so each field states its name and its type where the record is declared.

```sym-error
type Chord:
    root i32
```

`root` has no `:` before its type.

```sym unbuilt
type Chord:
    root: i32
```
