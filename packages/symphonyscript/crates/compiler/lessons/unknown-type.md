# unknown-type

A name in a type is an enum of the vocabulary. (SPEC §3.2)

The source declares no types of its own, so every named type comes from the vocabulary.

```sym-error
input a: Shape = 1
```

The vocabulary declares no enum `Shape`.

```sym
input a: Wave = Wave::Saw
```
