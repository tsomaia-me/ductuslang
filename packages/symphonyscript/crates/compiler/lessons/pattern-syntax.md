# pattern-syntax

A pattern is `pattern`, a name, its parameters in parentheses, `:` and an indented body of `let`s and then placements. (SPEC §6.4)

A pattern is used by its name, and its body is what a use expands to.

```sym-error
pattern (x):
    Lamp
```

The pattern has no name.

```sym
pattern Wash(x):
    Lamp/x
```
