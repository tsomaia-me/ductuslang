# generic-syntax

Bracket parameters follow a declaration's name in brackets, each a name, or `const` and a name, with an optional constraint after `:`; a type's bracket arguments follow its name in brackets, each a type or a compile-time expression. (SPEC §6.8)

The brackets name what a call or a use binds from its arguments, so each entry is one name.

```sym-error
fn first[T,](xs: T[3]): xs[0]
```

The list ends at `,` with no parameter after it.

```sym unbuilt
fn first[T](xs: T[3]): xs[0]
```
