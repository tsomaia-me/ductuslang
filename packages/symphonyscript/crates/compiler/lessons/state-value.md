# state-value

A cell may hold an array, a string, a record, a sum, a map or a document, never a node reference or a function. (SPEC §6.1)

A cell's value changes at runtime, and node references and functions exist only at compile time.

```sym-error
Clip c:
    Note
state s = c
```

The cell holds a node reference.

```sym
state s = 1
```
