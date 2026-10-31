# unexpected-indent

A line indents deeper than the line before it only where that line opens a block. (SPEC §2.2)

Indentation means nesting, so an indented line under a line that opens no block nests in nothing.

```sym-error
state a(1)
    state b(2)
```

`input b` is indented under a line that opens no block.

```sym
state a(1)
state b(2)
```
