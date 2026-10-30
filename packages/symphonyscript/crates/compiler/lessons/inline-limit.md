# inline-limit

A compilation inlines at most 1,000,000 calls of functions that are not recursive, a call counting each time the body holding it is inlined or expanded, and a `for` expression's body counting once however many times it runs. (SPEC §4.6)

Each call copies its function's body, so nested calls can multiply without end.

```sym-error
fn h0(x): x
fn h1(x): h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x)
fn h2(x): h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x)
fn h3(x): h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x)
fn h4(x): h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x)
fn h5(x): h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x)
fn h6(x): h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x)
Clip k:
    Note/(h6(1))
```

`h6(1)` inlines ten million calls.

```sym
fn h0(x): x
fn h1(x): h0(x) * 10
Clip k:
    Note/(h1(1))
```
