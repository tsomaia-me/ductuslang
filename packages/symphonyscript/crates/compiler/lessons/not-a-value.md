# not-a-value

A destination names an instance; it is never an inline placement or a pattern use. (SPEC §14)

A wire connects instances that exist elsewhere.

```sym-error
pattern V():
    Cue
Scene s:
    Triggers: V
```

`V` is a pattern.

```sym
Scene v:
    Cue
Scene s:
    Cue
    Triggers: v
```
