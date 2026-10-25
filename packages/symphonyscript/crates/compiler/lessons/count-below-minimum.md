# count-below-minimum

Each top-level kind, each parent's children of each kind and each instance's wires of each kind meet the vocabulary's minimum count. (SPEC §10.2)

The vocabulary states which instances a program needs.

```sym-error bounded
Root r:
    Link: r
```

`Root` needs at least one `Item` child.

```sym bounded
Root r:
    Item
    Link: r
```
