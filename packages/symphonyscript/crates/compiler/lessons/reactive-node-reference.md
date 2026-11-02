# reactive-node-reference

A value holding node references that reads a cell is an error. (SPEC §6.2)

Node references are fixed when the program compiles; a destination chosen at runtime is written in the wire itself.

```sym-error
state i(0)
fn pick(k):
    let t = [dusk, night][k]
    k
Scene dusk:
    Cue
Scene night:
    Cue
Scene s | hold=(pick(i)):
    Cue
```

`t` chooses an instance by the parameter `k`, which reads the cell `i`.

```sym
state i(0)
Scene dusk:
    Cue
    Triggers: [dusk, night][i]
Scene night:
    Cue
```
