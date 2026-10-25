# missing-attribute

Leaving unset an attribute the vocabulary gives no default is an error. (SPEC §7.6)

The instance would have no value for it.

```sym-error
Kit k:
    Sampler
```

`file` has no default.

```sym
Kit k:
    Sampler/"kick.wav"
```
