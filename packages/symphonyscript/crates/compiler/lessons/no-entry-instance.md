# no-entry-instance

A program with no instance of an entry kind draws a warning. (SPEC §10.2)

The entry is where the program starts, so without one nothing runs.

```sym-error
Clip lonely:
    Note
```

The program places no `Session`.

```sym
Session s:
    Track as t:
        Starts: lonely
Clip lonely:
    Note
```
