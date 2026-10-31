# reactive-node-reference

A `let` holding node references that reads an input is an error. (SPEC §6.2)

Node references are fixed when the program compiles; a destination chosen at runtime is written in the wire itself.

```sym-error
state i(0)
let t = [verse, chorus][i]
Clip verse:
    Note
Clip chorus:
    Note
```

`t` chooses an instance by the input `i`.

```sym
state i(0)
Clip verse:
    Note
    Plays: [verse, chorus][i]
Clip chorus:
    Note
```
