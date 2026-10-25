# dedent-mismatch

A line that indents less than the line before it lands exactly on the column of an enclosing line. (SPEC §2.2)

The column decides which block a line belongs to, so a column between two blocks belongs to neither.

```sym-error
Clip c:
    Note
  Note
```

The last line stops at column 2, which no enclosing line uses.

```sym
Clip c:
    Note
    Note
```
