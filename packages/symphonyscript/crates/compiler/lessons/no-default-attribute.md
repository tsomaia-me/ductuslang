# no-default-attribute

`/expr` on a kind without a default attribute is an error. (SPEC §7.5)

`/expr` sets the default attribute, and the kind has none.

```sym-error
Synth s:
    Mixer/1
```

`Mixer` has no default attribute.

```sym
Synth s:
    Osc/440
```
