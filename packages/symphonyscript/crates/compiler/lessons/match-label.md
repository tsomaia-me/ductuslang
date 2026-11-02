# match-label

An arm's label is a variant of the scrutinee's enum, or a literal of its type and unit family. (SPEC §5.4)

A label of another type could never match.

```sym-error
state m(Mode::Manual)
state l = match m:
    Purple: 1
    else: 2
```

`Mode` has no variant `Purple`.

```sym
state m(Mode::Manual)
state l = match m:
    Manual: 1
    else: 2
```
