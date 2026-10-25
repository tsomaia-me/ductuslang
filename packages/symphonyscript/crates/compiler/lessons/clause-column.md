# clause-column

A continued attribute clause lines up its entries in one column; a line that misses it by the display width of wide characters is an error that shows the column. (SPEC §2.2)

Columns count characters, and a wide character looks two columns wide while counting as one.

```sym-error
Clip 音楽 | gain=1
            level=2
```

`level` is aligned by eye with `gain`, but the clause's column counts the two wide characters as one column each.

```sym
Clip 音楽 | gain=1
          level=2
```
