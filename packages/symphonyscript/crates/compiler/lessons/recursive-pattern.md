# recursive-pattern

No pattern reaches itself through uses. (SPEC §6.4)

Its expansion would never end.

```sym-error
pattern A():
    Plant:
        A
```

`A` uses itself.

```sym
pattern A():
    Plant:
        Valve
```
