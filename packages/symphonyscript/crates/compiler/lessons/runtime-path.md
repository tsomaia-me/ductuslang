# runtime-path

A path whose start is chosen at runtime is an error. (SPEC §5.6)

A path is resolved when the program compiles.

```sym-error
input i = 0
let s = [verse, chorus]
Clip verse | gain=s[i].gain:
    Note
Clip chorus:
    Note
```

`s[i]` is chosen by the input `i`.

```sym
let s = [verse, chorus]
Clip verse | gain=s[1].gain:
    Note
Clip chorus:
    Note
```
