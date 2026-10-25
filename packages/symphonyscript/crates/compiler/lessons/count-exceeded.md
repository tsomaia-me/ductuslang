# count-exceeded

The number of instances of each kind at top level, among a parent's children and among an instance's wires is within the vocabulary's count. (SPEC §10.2)

The vocabulary states how many instances each place holds.

```sym-error
Session a
Session b
```

At most one `Session` stands at top level.

```sym
Session a
```
