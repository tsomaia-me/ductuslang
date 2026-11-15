# What `rustc` costs on a module shaped like generated code

Taken by the peer on 2026-11-15, on `wb` (the slow host, 18 cores, `rustc 1.94.0`), before the first question on the compile mode was put to the owner. It is a stand-in and no design: no generator exists, so the module's shape is a guess. It sizes one thing only, the order of the wait a user has when every edit goes through `rustc`.

## What was compiled

One Rust file, one library crate, no dependency. For N cells: N functions of the form `fn cell_k(s: &mut St) -> bool`, each marked `#[inline(never)]`, each reading two or three slots of two vectors, computing a few float or saturating integer operations (one in four holds a loop of eight turns and a `sqrt`), comparing the result with the stored bits and storing it; and one static table of the N function pointers. Compiled with `rustc --edition 2021 --crate-type lib`, once plain and once with `-O`, under `nice -n 19`, one run each.

## The times, on `wb`

| Cells | Source | Plain | `-O` |
|---|---|---|---|
| 1,000 | 0.2 MB | 1.3 s | 3.8 s |
| 10,000 | 2.1 MB | 11.2 s | 38.8 s |

About 1.1 ms a cell plain and 3.9 ms a cell with `-O`, growing in line with the count. Linking a program, starting it and the SymphonyScript compiler's own time come on top. `wb` is slow; no time was taken on `wb2` or on a user's machine, so these are an order of size and never a target.

## The generator

```python
import random
random.seed(1)
def gen(n):
    out=["#![allow(dead_code)]","pub struct St { pub f: Vec<f32>, pub i: Vec<i64> }"]
    for k in range(n):
        a=random.randrange(n); b=random.randrange(n); c=random.randrange(n)
        kind=k%4
        if kind==0:
            body=f"let v = s.f[{a}] * 0.8 + s.f[{b}].min(1.0); if v.to_bits() != s.f[{k}].to_bits() {{ s.f[{k}] = v; true }} else {{ false }}"
        elif kind==1:
            body=f"let v = if s.i[{a}] > 60 {{ s.f[{b}] }} else {{ s.f[{c}] * 0.5 }}; if v.to_bits() != s.f[{k}].to_bits() {{ s.f[{k}] = v; true }} else {{ false }}"
        elif kind==2:
            body=f"let v = s.i[{a}].saturating_add(s.i[{b}]).saturating_mul(3); if v != s.i[{k}] {{ s.i[{k}] = v; true }} else {{ false }}"
        else:
            body=f"let mut acc = 0.0f32; for j in 0..8usize {{ acc += s.f[({a} + j) % {n}] * (j as f32); }} let v = acc.sqrt(); if v.to_bits() != s.f[{k}].to_bits() {{ s.f[{k}] = v; true }} else {{ false }}"
        out.append(f"#[inline(never)] pub fn cell_{k}(s: &mut St) -> bool {{ {body} }}")
    out.append(f"pub static RULES: [fn(&mut St) -> bool; {n}] = [")
    out.append(",".join(f"cell_{k}" for k in range(n)))
    out.append("];")
    return "\n".join(out)+"\n"
for n in (1000,10000):
    open(f"gen_{n}.rs","w").write(gen(n))
```
