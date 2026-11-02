# inline-placement

A placement with a gate or an attribute clause is parenthesized or written on its own line. (SPEC §7.7)

The clause would otherwise run on into the placements after it.

```sym-error
Rig r:
    Lamp/0.5 Lamp | hue=1
```

The second placement's clause shares the line unparenthesized.

```sym
Rig r:
    Lamp/0.5 (Lamp | hue=1)
```
