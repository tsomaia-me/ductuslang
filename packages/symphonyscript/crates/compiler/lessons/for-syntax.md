# for-syntax

A loop head is `for`, one name or two separated by `,`, each with an optional annotation, `in` and an iterable (SPEC §7.9). A placement `for` ends its head with `:`, and a `for` expression's head goes on with `into`, the accumulator's name, `from`, the start and `:` (SPEC §5.10). A fault in either head draws this code.

The names are the loop's variables its body reads, and the head says what the body runs over.

```sym-error
Clip c:
    for in 0..2:
        Note
```

The placement `for` has no name: write `for <name> in <iterable>:`.

```sym
Clip c:
    for i in 0..2:
        Note
```

```sym-error
let total = for x in [1, 2] from 0: x
```

The `for` expression names no accumulator: write `for <name> in <iterable> into <name> from <start>:`.

```sym
let total = for x in [1, 2] into sum from 0: sum + x
```
