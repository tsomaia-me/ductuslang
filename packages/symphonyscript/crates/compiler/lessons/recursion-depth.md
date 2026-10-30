# recursion-depth

Compile-time evaluation deeper than the nesting limit, 1,024 nested calls by default, is an error. (SPEC §6.3)

The limit stops a recursion that never reaches its base case.

```sym-error
fn f(n): if n < 1 then 0 else f(n - 1) + 1
Clip c:
    Note/(f(2000))
```

The call nests 2,000 calls deep.

```sym
fn f(n): if n < 1 then 0 else f(n - 1) + 1
Clip c:
    Note/(f(20))
```
