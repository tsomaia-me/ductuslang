# recursive-pattern

No pattern reaches itself through uses. (SPEC §6.4)

Its expansion would never end.

```sym-error
pattern A():
    Clip:
        A
```

`A` uses itself.

```sym
pattern A():
    Clip:
        Note
```
