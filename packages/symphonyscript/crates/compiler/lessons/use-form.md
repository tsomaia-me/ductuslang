# use-form

A pattern use carries no flags and no body. (SPEC §6.4)

Flags belong to a kind's attributes, and the pattern's body is the use's body.

```sym-error
pattern P():
    Note
Clip c:
    P'
```

The use carries a flag.

```sym
pattern P():
    Note'
Clip c:
    P
```
