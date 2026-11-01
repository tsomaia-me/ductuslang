# reactive-node-reference

A value holding node references that reads a cell is an error. (SPEC §6.2)

Node references are fixed when the program compiles; a destination chosen at runtime is written in the wire itself.

```sym-error
state i(0)
fn pick(k):
    let t = [verse, chorus][k]
    k
Clip verse:
    Note
Clip chorus:
    Note
Clip c:
    Note/(pick(i))
```

`t` chooses an instance by the parameter `k`, which reads the cell `i`.

```sym
state i(0)
Clip verse:
    Note
    Plays: [verse, chorus][i]
Clip chorus:
    Note
```
