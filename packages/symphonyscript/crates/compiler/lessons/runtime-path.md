# runtime-path

A path whose start is chosen at runtime is an error. (SPEC §5.6)

A path is resolved when the program compiles.

```sym-error
state i(0)
let s = [dusk, night]
Scene dusk | hold=s[i].hold:
    Cue
Scene night:
    Cue
```

`s[i]` is chosen by the input `i`.

```sym
let s = [dusk, night]
Scene dusk | hold=s[1].hold:
    Cue
Scene night:
    Cue
```
