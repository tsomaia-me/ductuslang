# Architecture (draft)

Ruled, and not yet reviewed or approved as final text. Reviewed and approved, it becomes the architecture's documents.

- A contract is defined against an interface, and the implementation is passed in.
- The Program is the only hand-off.
- Generated code meets the runtime through a narrow, stated seam.
- Hosts and consumers are written against an interface, and not against the runtime itself.
- These three hold whether or not a second runtime is ever written.
- Every rule is written once. The operations on values and the commit's rules live in one place. Compile-time evaluation, the interpreter and the generated Rust all call them. Equality bit for bit then holds by construction, and a test over every fixture guards the wiring.
- A device carries the runtime and the generated code, and no compiler.
- Tests of running behaviour run in both modes.
- A contract's exact shape is written once, in code, as interfaces with no implementation inside. Its meaning stays in the specifications.
- The architecture is several documents: a short top document with the map, the contracts and the rules that bind every part; one for the compiler; one for the runtime; one for each mode.
