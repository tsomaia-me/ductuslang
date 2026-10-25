# inline-placement

A placement with a gate or an attribute clause is parenthesized or written on its own line. (SPEC §7.7)

The clause would otherwise run on into the placements after it.

```sym-error
Clip c:
    Note/60 Note | velocity=1
```

The second placement's clause shares the line unparenthesized.

```sym
Clip c:
    Note/60 (Note | velocity=1)
```
