# Modes (draft)

Ruled, and not yet reviewed or approved as final text. Reviewed and approved, it becomes the modes' specifications.

- A mode is when the Program meets the runtime.
- The interpreted mode: the Program arrives as data, at load. The runtime walks it and can take a new one, which is reload.
- The compiled mode: the Program is turned into Rust and built together with the runtime. Replacing it needs a new build.
- One Program, two takers. The compiler ends at the Program in both modes. The interpreter loads it and the generator translates it. Neither reads the source.
- The compiled mode is the main mode. The interpreted mode is the secondary one, a convenience, and it is built to the same bar: efficient, optimized, well planned and well architected.
- The edit loop runs in the interpreted mode, with live reload. Generated Rust is what ships and what every speed promise is stated for, and it never reloads.
- Both modes give the same results, bit for bit.
- A mode names no runtime. The same mode can run on different runtimes.
- Nothing is promised about a Program travelling to a runtime elsewhere. The Program is kept plain data, so that it stays possible.
- The SPEC names no mode. Its §12 and §13 hold for every mode. Each mode has a specification of its own, which says only what differs: when the Program meets the runtime, whether it offers reload, what a host is written against, and what it promises.
- A runtime has no specification of its own. SPEC §12 and §13 are it.
