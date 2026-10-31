# reactive-initializer

A cell's init is a compile-time value. (SPEC §6.1)

At load a host-fed cell holds its init and a derived cell with an init starts from it, before any cell steps; a start taken from another cell is written in the rule.

```sym-error
state a(1)
state b(a)
```

`b`'s init reads the cell `a`.

```sym
let start = 1
state a(start)
state b(start)
```
