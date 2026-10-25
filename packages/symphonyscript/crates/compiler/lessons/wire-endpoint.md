# wire-endpoint

Every wire's source and every candidate destination are kinds its wire kind connects. (SPEC §10.2)

The vocabulary states what each wire kind connects.

```sym-error
Track t:
    Starts: t
```

`Starts` connects to a clip, and `t` is a track.

```sym
Clip c:
    Note
Track t:
    Starts: c
```
