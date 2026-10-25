# match-label

An arm's label is a variant of the scrutinee's enum, or a literal of its type and unit family. (SPEC §5.4)

A label of another type could never match.

```sym-error
input w = Wave::Saw
let l = match w:
    Purple: 1
    else: 2
```

`Wave` has no variant `Purple`.

```sym
input w = Wave::Saw
let l = match w:
    Saw: 1
    else: 2
```
