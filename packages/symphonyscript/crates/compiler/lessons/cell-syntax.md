# cell-syntax

A cell is `state`, `stream` or `channel`, an optional capacity `[N]`, a name, an optional init in parentheses, an optional type and an optional clock, then `=` and its rule or the end of the line; a declaration with an init and no rule is host-fed, one with a rule is derived, and a declaration with neither is an error. Cells are declared at top level. (SPEC §6.1)

The init gives a cell the host writes its value and type, and the rule computes a cell the program derives.

```sym-error
state level
```

`level` has neither an init nor a rule.

```sym
state level(0)
```
