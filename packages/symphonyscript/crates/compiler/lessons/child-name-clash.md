# child-name-clash

A child or wire name equal to a name of its own or an enclosing body, to an attribute of its parent's kind, or to a pattern parameter, a pattern `let` or a `for` variable visible where it is declared is an error. (SPEC §6.5)

The name would hide the other one inside the body.

```sym-error
Plant p:
    Valve as dup
    Valve as dup
```

`dup` names two children.

```sym
Plant p:
    Valve as feed
    Valve as vent
```
