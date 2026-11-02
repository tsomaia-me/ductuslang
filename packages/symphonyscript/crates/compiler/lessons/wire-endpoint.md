# wire-endpoint

Every wire's source and every candidate destination are kinds its wire kind connects. (SPEC §10.2)

The vocabulary states what each wire kind connects.

```sym-error
Plant p:
    Controller as c
    Controller:
        Drives: c
```

`Drives` connects to a valve, and `c` is a controller.

```sym
Plant p:
    Valve as v
    Controller:
        Drives: v
```
