# for-variable-clash

A `for` variable equal to a visible name is an error, and so is a `for` expression's loop name, its accumulator or its block form's local `let` equal to a visible name. (SPEC §6.5)

The name would hide the visible one in the loop's body.

```sym-error
Clip c:
    for sin in 0..2:
        Note
```

`sin` is a built-in.

```sym
Clip c:
    for step in 0..2:
        Note
```
