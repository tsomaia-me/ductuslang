# wire-cycle

No cycle of wires contains a wire of a kind the vocabulary does not mark cyclic. (SPEC §8.4)

Signals that flow in a cycle through such wires would never settle.

```sym-error
Synth s:
    Filter as b:
        Feeds: c
    Filter as c:
        Feeds: b
```

`b` feeds `c` and `c` feeds `b`.

```sym
Synth s:
    Filter as b:
        Feeds: c
    Filter as c
```
