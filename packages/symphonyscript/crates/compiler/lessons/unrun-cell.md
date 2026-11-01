# unrun-cell

A derived cell with no clock that reads no cell but its own history is an error, and so is an unclocked cycle of cells that reads no cell outside itself. (SPEC §6.6)

A derived cell runs when a cell it reads steps; a cell that reads only its own past, or a cycle whose cells read only one another's pasts, has nothing that steps it, so nothing runs it.

```sym-error
state count(0) = count.past(1) + 1
```

`count` reads only its own history.

```sym
state hit(0)
state count(0) = if hit > 0 then count.past(1) + 1 else count.past(1)
```
