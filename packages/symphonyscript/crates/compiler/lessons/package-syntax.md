# package-syntax

A package file holds the `Package` record literal, `require` with a source in quotes and an optional `as` name, `use` with a library's name and an optional list of its names or `as` name, and `publish` with the names it publishes, each an optional `as` name after it. (SPEC §15.2)

A package takes in and gives out names only through these items, so each names a library or a name exactly.

```sym-error
require util
```

The source of a library is a string.

```sym unbuilt
require "../util"
```
