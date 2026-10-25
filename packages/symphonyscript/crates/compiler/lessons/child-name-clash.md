# child-name-clash

A child or wire name equal to a name of its own or an enclosing body, to an attribute of its parent's kind, or to a pattern parameter, a pattern `let` or a `for` variable visible where it is declared is an error. (SPEC §6.5)

The name would hide the other one inside the body.

```sym-error
Clip c:
    Note as dup
    Note as dup
```

`dup` names two children.

```sym
Clip c:
    Note as first
    Note as second
```
