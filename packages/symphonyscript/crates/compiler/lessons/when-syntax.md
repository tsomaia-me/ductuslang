# when-syntax

A `when` block is `when`, its condition, `:` and an indented block, with an optional `else:` block. (SPEC §9.2)

The condition gates the block's placements.

```sym-error
Clip c:
    when x
        Note
```

The condition has no `:`.

```sym
state x(true)
Clip c:
    when x:
        Note
```
