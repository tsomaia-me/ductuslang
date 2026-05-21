# Ductus Language Specification

**Status:** Draft v0.1. Living document. Working reference for the language's
design. Pairs with `GRAMMAR.md` (lexical and syntactic structure) and
`Topics.md` (decision log).

---

## 1. Introduction

### 1.1 Purpose

This specification is the authoritative source for Ductus's type system,
evaluation model, and runtime semantics. Implementation details (compiler
internals, optimizations, runtime representation) are out of scope except where
they constrain user-visible semantics. The grammar of the language is specified
separately in `GRAMMAR.md`; this document refers to grammar productions where
relevant and does not duplicate them. Where the grammar and this specification
appear to conflict, this specification is authoritative; the grammar is to be
revised to match.

### 1.2 Status

The specification is under active development. The type system (this section
and §§2–10) is fully specified except for object safety (§5.2.4) and coercion
semantics (§5.2.5), which are deferred to a future revision. The reactive
system, runtime semantics, and the node/connection composition model are
partially specified or deferred. Sections labeled "deferred" indicate
decisions consciously postponed for later refinement.

### 1.3 Design Philosophy

Ductus is a general-purpose, statically typed language designed to make
compositional reactive systems first-class. The language commits to no
domain-specific primitives.

The language is built on a small set of load-bearing principles:

**Strong static types with minimal ceremony.** Every value has a concrete type
known to the compiler at code generation time. Most types are inferred from
context. Annotations are required only where inference is ambiguous or where
the user wants to pin a choice deliberately.

**Immutability by default; isolated local mutation as escape hatch.**
External state — module-level declarations, type definitions, signals,
function parameters, record fields as a property of types — is always
immutable. There is no module-level `mut`, no globally-mutable state.
Inside function bodies, controlled local mutation is available through
`mut` bindings (§11). Mutation is bounded to the function body that
declared it; callers never observe a function's internal mutations
except through its declared return value. Time-varying *external*
behavior is expressed through the reactive system, not through
assignment.

**Single ownership.** Every value has exactly one owner at any moment.
Passing a non-`Copy` value to a function transfers ownership; the
caller's binding is consumed. Returning a value transfers ownership back
to the caller. Read-only access to a non-`Copy` value without ownership
transfer is provided through call-scoped borrows (`&T` parameters) per
§11.9. There is no garbage collector, no reference counting at the
language level (the runtime may use refcounting internally for specific
types like `string` per §11.6), and no shared mutable state.

**Effectively pure functions.** From a caller's perspective, every
user-defined function is referentially transparent: same inputs produce
the same outputs, with no externally observable side effects on the
caller's bindings beyond the function's declared return value. A
function may use `mut` bindings, indexed assignment, and `while`/`for`
loops *internally* (§11, §12), but these are implementation details
invisible at the call site. The reactive system provides the controlled
mechanism for time-varying behavior across the program; ordinary
computation is pure-by-contract.

**Compile-time evaluation where possible.** Pure functions and immutable
bindings together mean that any expression not involving a signal or external
input is compile-time evaluable. The language uses this aggressively for
type-level computation, value-fits-type checking, and dependent-ish typing.

**Nominal types.** Records, enums, traits, nodes, and connections are nominal —
identity is by name, not structure. Tuples and trait-constraint intersections
are explicit structural carve-outs with clear semantics.

**Traits as the abstraction mechanism.** Behavior abstraction uses nominal
traits with explicit `satisfies`/`fulfill` declarations. Coherence is enforced
structurally via orphan rules.

**Uniform function call syntax.** Methods and free functions are
interchangeable syntactic forms for the same underlying operation: a
function `f(x, y)` may equivalently be called as `x.f(y)`. Records
carry data; functions carry behavior; the call site chooses the form
that reads best. Operator application uses the `|>` token (§13.17);
`>>` is bitwise right shift only.

**Two-track failure model.** Failures are either bugs (traps, process abort, no
catch mechanism) or recoverable conditions (`Option`/`Result` values, `?`
propagation). The choice is made at the operation site, not retroactively.

### 1.4 Conventions

Code examples use Ductus syntax per `GRAMMAR.md`. Type-name case conventions:

- Concrete primitive types: lowercase (`i32`, `f64`, `bool`, `char`, `string`, `never`).
- Built-in placeholder keywords: lowercase (`numeric`, `integer`, `float`, `signed`, `unsigned`).
- Trait names: PascalCase (`Numeric`, `Add`, `Display`, `Ord`).
- User-defined type names: PascalCase (`Vec3`, `Person`, `Event`).
- Identifiers (functions, variables, fields): snake_case (`full_name`, `first_name`).

**Keywords are always lowercase.** No keyword has a capitalized form.
This includes all declaration keywords (`node`, `connection`, `trait`,
`type`, `fn`, `operator`, `effect`, `signal`, `attr`, `recurrent`,
`derived`, `stream`, `sink`, `const`, `let`, `mut`), all clause
keywords (`parts`, `in`, `out`, `expose`, `when`, `satisfies`,
`fulfill`, `default`, `from`, `to`, `pairs`, `on`, `where`,
`desired`, `observed`, `ring`, `gate`), all control-flow keywords
(`if`, `else`, `match`, `for`, `while`, `break`, `continue`,
`return`), and all operator-context keywords (`as`, `is`, `and`,
`or`, `not`). The rule is normative and takes precedence over any
conflicting grammar.

Identifier character set: identifiers may contain `#` as a leading,
infix, or terminating character — for example `#default`,
`audio#main`, `note#`. The `#` character behaves like a letter for
identifier purposes; it is a valid identifier character in every
position. Precise lexical rules are in `GRAMMAR.md`; this rule is
normative and takes precedence over any conflicting grammar.

"The compiler" refers to the implementation's static analysis phase. "Runtime"
refers to execution. "Codegen" refers to the boundary at which all types must
be concrete.

---

## 2. Type System Foundations

This section specifies the fundamental machinery of the type system: how types
flow through a program (placeholders), when they are checked (inference and
definition-time analysis), how they are realized in code (monomorphization),
and how compile-time evaluation participates in the type system.

### 2.1 Placeholders

The type system contains a category of compile-time-only types called
*placeholders*. A placeholder represents a type that has not yet been resolved
to a concrete type. Placeholders exist solely during type checking and are
eliminated before code generation. Every runtime value has a concrete machine
type; the placeholder mechanism is the compiler's machinery for determining
what that concrete type is.

#### 2.1.1 Placeholders attach to values, not bindings

A placeholder is a property of a *value*. When a value with a placeholder type
flows through a binding, the binding carries the placeholder forward
transparently:

```
let x = 5
```

The literal `5` produces a value with an integer placeholder. The binding `x`
is a transparent alias for that value; `x` itself does not have a type
independent of its value.

#### 2.1.2 Resolution at use sites

A placeholder is resolved to a concrete type at a *use site* — a position in
the code where the value participates in an operation requiring a concrete
type. Use sites include:

- An explicit type annotation (`let y: i32 = x`).
- An argument to a function parameter with a concrete type.
- An operand to an operator whose other operand is concretely typed.
- A field assignment in a record where the field type is concrete.
- A return value of a function with a concrete return type.

Each use site resolves the placeholder independently. The same placeholder-
bearing value used at two different use sites can resolve to two different
concrete types:

```
let x = 5
let a: i32 = x      // x resolves to i32 here
let b: i64 = x      // x resolves to i64 here
```

This is sound because the value `5` is compile-time known (§2.4) and the
compiler verifies it fits both target types.

Resolution at a use site considers all information available at that site,
including other operands' types in the same expression and the value-fits
check from §2.4.3. The integer/float kind of a placeholder tags it for
defaulting (§3.1.5) but does not prevent resolution to a compatible type of
a different kind, provided the value fits exactly:

```
let x = 5
let f: f64 = x         // ✓ integer placeholder resolves to f64; value 5 fits exactly
let g: f32 = x         // ✓ same; value 5 fits exactly in f32
let h = x * 1.5_f32    // ✓ integer placeholder resolves to f32 in mixed-kind expression
                       //   per the placeholder cross-kind resolution rule (§2.1) and
                       //   the value-fits-type check (§2.4.3); value 5 fits exactly in f32
```

A binding whose right-hand side is itself an expression with a placeholder
follows the same rule applied to the expression: the expression resolves first
using its own context (operand types, value-fits, defaulting), and the binding
adopts the resolved type. The binding's annotation status (or absence) does
not provide context to the expression; resolution flows forward from
expression to binding, not backward.

#### 2.1.3 Bindings to concretely-typed expressions

When a binding's right-hand side has a concrete type, the binding carries that
concrete type. There is no placeholder to propagate:

```
let x: i32 = 5             // x is i32
let y = some_function()    // y is whatever some_function() returns, concretely
```

#### 2.1.4 Resolution failures

A use site that cannot resolve its placeholder produces a compile error
*at the use site*. Errors are local to the use site, not propagated back to the
binding. Unused bindings (dead code) require no resolution and produce no error
from the placeholder mechanism, though they may produce a separate
unused-binding warning.

A single value used at multiple use sites that demand mutually incompatible
concrete types is also a use-site issue, surfaced as errors at the conflicting
sites.

#### 2.1.5 No first-class runtime placeholders

The placeholder is strictly compile-time. No value at runtime has placeholder
type; no machine representation corresponds to a placeholder. This forecloses
dynamic typing in disguise and keeps memory layout, dispatch costs, and
codegen output predictable.

### 2.2 Type Inference

The compiler infers types for variables, function parameters, function return
types, and generic type parameters using a bidirectional inference algorithm.
Users provide annotations where necessary; the compiler fills in the rest.

#### 2.2.1 Inference mechanism

Inference operates within function bodies and across function signatures (for
generic instantiation). At a high level, the algorithm:

1. Treats every omitted type annotation as a fresh placeholder.
2. Generates constraints from the expression structure (operator types,
   function signatures, field types, etc.).
3. Solves the constraints, resolving placeholders to concrete types or to
   trait-constrained type variables.
4. Reports errors at sites where constraints cannot be satisfied.

#### 2.2.2 Definition-time body checking

Generic function bodies are typechecked at definition, not deferred to call
sites. The compiler analyzes the body's operations to determine the constraints
on the generic parameters:

```
fn lerp(a, b, c):
  a + (b - a) * c
```

From the body, the compiler infers that `a`, `b`, and `c` must support `+`,
`-`, and `*`. The inferred constraints are attached to the function's
signature; call sites must satisfy them.

Definition-time checking gives error locality (bugs in the body point at the
body, not at call sites), enables isolated verification of generic functions
(a generic function is valid before any call exists), and supports tooling for
uncalled generics.

#### 2.2.3 Implicit and explicit generics are equivalent

A function with omitted parameter types is generic. Each omitted parameter type
becomes a *distinct* fresh generic parameter. The implicit form desugars
mechanically to the explicit form, one fresh parameter per omitted slot:

```
fn lerp(a, b, c): a + (b - a) * c
// initial desugaring (before inference):
fn lerp[T0, T1, T2](a: T0, b: T1, c: T2): a + (b - a) * c
```

The desugaring produces three distinct type parameters because there are three
omitted parameter types. Inference (§2.2.1) then generates constraints from the
body's operations and may unify some parameters with each other if the body's
operations force them to be the same type. For `lerp`, the body `a + (b - a) *
c` constrains the parameters such that inference may unify them into one — but
the unification is a *result* of inference, not part of the desugaring.

For a function whose body does not relate its parameters, the synthesized
parameters remain distinct:

```
fn pair(a, b): (a, b)
// desugars to (and stays as):
fn pair[T0, T1](a: T0, b: T1) -> (T0, T1)
```

Here `a` and `b` are not connected by any operation, so `T0` and `T1` remain
independent generic parameters and the function is genuinely generic in two
parameters.

The implicit and explicit forms produce the same code and the same semantics.
The choice between them is stylistic. Mixed forms are permitted: some
parameter types explicit, others omitted, with the omitted ones receiving
fresh parameters per the same rule.

#### 2.2.4 Trait-based constraints

Inferred constraints reference traits (§3). Operations in the body resolve to
trait methods (`+` resolves to `Add::add`, where `Add` denotes `Add[Self]`
per §3.1.6's default-type-parameter resolution), and the relevant trait
becomes the constraint on the corresponding parameter. The trait system's
umbrella traits (§3.6) let the compiler simplify inferred constraint sets for
readability: `Add + Sub + Mul + Neg + Zero + One` may collapse to `Numeric`
when unambiguous.

#### 2.2.5 Type-argument inference at call sites

When a generic function is called, the compiler infers the type arguments from
the call's argument types where possible. Explicit type arguments are an
opt-in fallback using turbofish syntax `::[T]`:

```
let r = lerp(0.0_f64, 1.0_f64, 0.5_f64)        // T inferred from arguments
let r = lerp::[f64](0.0, 1.0, 0.5)             // T explicit
```

The `::` prefix on the type-argument list disambiguates from indexing
(see `GRAMMAR.md` §3.15–§3.16). Without `::`, `foo[T](args)` is ambiguous
between "index `foo` with `T`, then call" and "call generic `foo` with type
argument `T`". The `::` forces path-navigation mode where `[T]` is
unambiguously a type-argument list.

#### 2.2.6 Type wildcards

The identifier `_` in a type-annotation position denotes a type that the
compiler should infer from context. It is a placeholder per §2.1, resolved
at its use site by the surrounding type information.

```
let r: Result[i32, _] = compute()       // i32 pinned; error type inferred
let v: Vec[_] = make_ints()             // element type inferred
let pair: (_, string) = build()         // tuple's first component inferred
```

The wildcard is permitted in any type-expression position: generic
arguments, tuple components, function return types in annotations,
trait-bound positions where inference can resolve the constraint, and
others. If the compiler cannot infer the type at the wildcard's site from
the surrounding context, the resulting error is reported at the wildcard's
location, identifying the inference failure and what context was missing.

The wildcard is distinct from the pattern wildcard `_` (used in pattern
matches per §6.2.4). They share the same character but appear in different
syntactic positions; the parser disambiguates by position.

### 2.3 Monomorphization

Generic functions are realized in code via monomorphization: each unique
combination of concrete type arguments produces a separate specialized function.
There is no type erasure, no boxing, no dynamic dispatch for static generic
calls. Users pay no runtime cost for generic abstraction beyond the cost of the
specialized operations themselves.

#### 2.3.1 Instantiation granularity

Each unique tuple of concrete type arguments at a call site produces a distinct
instantiation. `lerp(i32, i32, i32)` and `lerp(i32, i32, f64)` are separate
instantiations even if their machine code happens to be similar. Backend
codegen may deduplicate identical machine code as an optimization, but this
deduplication is semantically invisible.

#### 2.3.2 Cross-module instantiation

Monomorphization is per-call-site across module boundaries. A generic function
defined in module A and called from modules B and C with different concrete
types produces separate instantiations in each calling module. Consequence:
generic function bodies must be available (in source or intermediate
representation) to any module that calls them. Generic definitions are not
closed binary units from the linker's perspective. (This is a constraint on the
module system design, not on the type system.)

#### 2.3.3 Polymorphic recursion is forbidden

Polymorphic recursion — a recursive call within a generic function body that
would require a different type instantiation than the caller — is rejected at
compile time. Direct same-type recursion (the recursive call has the same type
arguments as the caller) is permitted and reuses the same instantiation.

This restriction guarantees that monomorphization terminates and produces a
finite set of instantiations. The use cases where polymorphic recursion is
genuinely needed (certain advanced data structures, functional patterns) are
out of scope for v1; explicit dynamic dispatch via `dyn` (§5) is available
for cases that require it.

#### 2.3.4 Dead code elimination

Dead code elimination operates per-instantiation. Each monomorphized variant is
independently eligible for elimination. A generic function with no call sites
produces no output. A generic function called with some type combinations but
not others produces exactly the instantiations called, nothing more. The
semantic unit for codegen is the instantiation, not the generic.

#### 2.3.5 Trait method dispatch in monomorphized code

Trait method calls in monomorphized code resolve to direct function calls to
the concrete implementation. There is no vtable, no indirection, no runtime
dispatch cost for static generic calls. Coherence (§3.7) guarantees
unambiguous resolution: exactly one `fulfill` block exists per (trait, type)
pair within the module graph.

Dynamic dispatch is available as an opt-in mechanism via `dyn` trait objects
(§5). This is the only path through which trait method calls incur runtime
indirection.

#### 2.3.6 Const-generic default expressions

Const-generic parameters may declare default values, including
*expressions* that reference other generic parameters of the same
declaration. The defaults are resolved at instantiation time
alongside type-parameter defaults.

```
operator merge[
  T,
  const N: usize = A.capacity + B.capacity,
](
  a: RingStream[T, A],
  b: RingStream[T, B],
) -> RingStream[T, N]
```

Here `N` defaults to `A.capacity + B.capacity` — an expression
referencing the const-generic values `A.capacity` and `B.capacity`
associated with the inferred type parameters. The default is
evaluated at instantiation time using the concrete arguments and
must produce a compile-time-known value of the declared type
(here, `usize`).

**Evaluation rules.**

- The default expression is evaluated in the const evaluation
  context (§2.4) at instantiation time.
- The expression may reference other generic parameters of the same
  declaration that precede it in the parameter list. Forward
  references are not permitted.
- The expression may reference projections of generic-parameter
  types that are const-generics (e.g., `A.capacity` where `A` is
  itself a type parameter that carries a capacity).
- The expression must produce a value of the declared parameter
  type. Compile error otherwise.

**Inference and override.**

At a call site, the const-generic parameter may be:
- Omitted entirely — the default expression is evaluated and used.
- Specified explicitly via turbofish — the explicit value overrides
  the default, subject to type-correctness.

```
// Use default: N = a.capacity + b.capacity
let m1 = merge(stream_a, stream_b)

// Override: N = 1024 (must be type-correct; the override is
// the caller's assertion that 1024 suffices)
let m2 = merge::<Event, 1024>(stream_a, stream_b)
```

**Type-parameter defaults** (§3.1.6.1) and const-generic defaults
follow the same evaluation order: the defaults are resolved in
left-to-right declaration order, each having access to the resolved
values of preceding parameters.

**No circular dependencies.** A default expression referencing a
later-declared parameter is a compile error. The dependency graph
among defaults is required to be a DAG; the compiler enforces this
during instantiation.

#### 2.3.7 Binary size

Monomorphization trades binary size for runtime performance and type
information preservation. For typical programs the tradeoff is favorable; for
programs with heavy generic instantiation across many type combinations, binary
size can grow. Mitigations available as later optimizations:

- Backend deduplication of identical machine code (already mentioned in
  §2.3.1; an implementation concern, not a semantic feature).
- Outlining of type-independent code into shared helpers.
- Opt-in dynamic dispatch via `dyn` trait objects, at the cost of indirection.

None of these change the language's semantic model. They are levers available
to implementations if binary size becomes a real constraint.

### 2.4 Compile-Time Evaluation

The language evaluates expressions at compile time whenever possible.
Compile-time-known values participate in type-level computation, value-fits-type
range checking, dependent-ish typing, and the elimination of runtime checks the
compiler can prove unnecessary. Compile-time evaluation is a semantic feature,
not an optimization: the language's design relies on it.

#### 2.4.1 Compile-time-known values

A value is *compile-time known* if its defining expression is compile-time
evaluable. The propagation rule is mechanical:

- Literals are compile-time known.
- Constructions over compile-time-known operands are compile-time known.
- Operations over compile-time-known operands are compile-time known.
- Calls to pure functions with compile-time-known arguments are compile-time
  known.
- Bindings of compile-time-known expressions are compile-time known.

Since all user-defined functions are pure (§1.3) and all bindings are
immutable, compile-time-knowability propagates freely through the expression
graph. The compiler determines compile-time-knowability automatically; users do
not annotate it for `let` bindings.

#### 2.4.1.1 `let` and `const` binding forms

The language has two binding forms:

- **`let`** — the general binding form. Immutable. The compiler determines
  compile-time-knowability automatically from the expression. A `let` bound
  to a non-reactive expression is compile-time known and may be tree-shaken,
  inlined, or otherwise optimized; a `let` bound to a reactive or runtime
  expression participates in the reactive graph and exists at runtime.
- **`const`** — a stricter binding form. The user *asserts* that the binding
  is compile-time-only; the compiler enforces this assertion and additionally
  guarantees the binding has no runtime existence whatsoever.

```
const PI = 3.14159
const TAU = 2.0 * PI            // derived from another const, also compile-time
const MAX_ITEMS: usize = 1024

let x = 5                       // compile-time known, but compiler decides what to do
let y = compute(input)          // compile-time known iff input is non-reactive
let z = read_sensor()           // reactive, runtime
```

#### 2.4.1.2 Semantics of `const`

A `const` binding has three properties beyond what `let` provides:

1. **Non-reactive guarantee.** The RHS must not involve any signal, derived
   value, external input, or reactive expression. Violation is a compile error
   at the `const` declaration site, identifying the source of reactivity. This
   makes intent visible at the binding site: readers see `const` and know,
   without scanning the RHS, that the value is purely compile-time.

2. **No runtime existence.** A `const` does not occupy a runtime memory
   location. Wherever it is referenced from non-`const` code, the value is
   inlined directly. Wherever it is referenced from another `const` or from
   type-level context, the value is used at compile time only. A `const` that
   is unreferenced (or referenced only from compile-time contexts whose results
   are themselves unused) does not appear in the compiled output at all.

3. **No addressability.** Because a `const` has no runtime location, it has
   no address. Operations that would require a runtime address (passing by
   reference, storing a pointer, FFI sharing) are compile errors. The `const`
   is a *value*, not a *location*.

#### 2.4.1.3 `const`-eligible types

A type is `const`-eligible if all of its values can be fully represented at
compile time, with no runtime allocation and no runtime state. The set
includes:

- All primitive types: `i8`–`i128`, `u8`–`u128`, `isize`, `usize`, `f32`,
  `f64`, `bool`, `char`, `string`, `never`.
- Fixed-size arrays whose element type is `const`-eligible.
- Records whose field types are all `const`-eligible.
- Enums (including payload-carrying) whose payload types are all
  `const`-eligible.
- Tuples whose component types are all `const`-eligible.
- Newtypes wrapping `const`-eligible types.

Types not `const`-eligible:

- Heap-allocated collection types (`Vec`, `HashMap`, etc.).
- Signal-bearing or reactive types.
- Types containing function references or closures with captured runtime state.
- `dyn` trait objects.

The compiler checks `const`-eligibility at the declaration site. A `const`
declaration whose RHS produces a non-`const`-eligible type is rejected with a
clear error identifying the offending type.

#### 2.4.1.4 `const` declaration sites

`const` is permitted at:

- Module top level — for shared constants and configuration values.
- Inside function bodies — for local compile-time-only values used in
  type-level positions (e.g., array sizes computed from arguments to a
  generic function).
- Inside type, trait, node, and connection bodies — for type-associated
  constants accessible via path syntax (`Vec3::ZERO`, `Color::WHITE`).

`const` declarations follow the same visibility model as other declarations
(§10): `public const TAU = ...`, `private const INTERNAL_THRESHOLD = ...`,
default `shared`.

#### 2.4.1.5 Relationship between `const` and `let`

The two forms coexist:

- A `let` bound to a non-reactive expression is *effectively* eligible for
  `const`-style optimization (tree-shaking, inlining), but the user has not
  asserted this and the compiler has not enforced it. The binding may or may
  not exist at runtime depending on whether anything observes it.
- A `const` is *guaranteed* not to exist at runtime, and the compiler enforces
  the non-reactive constraint. Users choose `const` to encode their intent
  and obtain the enforcement.

A `let` bound to an expression that uses `const` values is itself
compile-time known (constants propagate through pure expressions per §2.4.1).
There is no need to "promote" `let` to `const` for downstream `const` use; the
propagation rule covers it.

Tooling may suggest converting an eligible `let` to `const` as a stylistic
hint, but the compiler does not require it. The choice between the two forms
is the user's assertion about intent; the language does not infer the
assertion.

#### 2.4.2 Breaks in propagation

Two categories of expression are not compile-time known:

- Expressions involving *signals* (§13) or any reactive value derived from a
  signal. Signal values depend on the moment of evaluation and are inherently
  runtime.
- Expressions involving external I/O, host-boundary calls, or any future
  construct whose value is determined by the runtime environment.

Once a reactive or runtime dependency enters an expression's evaluation, the
expression and all derived values become runtime values. The propagation is
transitive: a function call whose argument includes a reactive value produces
a reactive result; a binding to a reactive expression is itself reactive.

#### 2.4.3 Value-fits-type checking

Compile-time-known values are checked against the type constraints they meet
in context. The compiler verifies that the value fits the demanded type and
produces a compile error if it does not:

```
let x: u8 = 200            // ✓ 200 fits in u8 (range 0..255)
let x: u8 = 300            // ✗ compile error: 300 doesn't fit u8
let x: i8 = -50            // ✓ -50 fits in i8 (range -128..127)
let x: u8 = -1             // ✗ compile error: -1 doesn't fit u8
let x: f32 = 5             // ✓ 5 exactly representable in f32
```

This applies to any compile-time-known value, however computed:

```
let y = 200
let z: u8 = y              // ✓ y is compile-time known as 200, fits u8
let w: u8 = y + 50         // ✓ compile-time evaluates to 250, fits u8
let v: u8 = y + 100        // ✗ compile-time evaluates to 300, doesn't fit u8

fn double(x: i32) -> i32: x * 2
let f = double(100)        // ✓ pure call, evaluates to 200
let g: u8 = f              // ✓ 200 fits u8
```

Integer values require exact fit. Float literal values fit any float type,
rounded to nearest representable. Integer-to-float fit follows the lossless
conversion rules (§4.5).

#### 2.4.4 Compile-time evaluation as type-level mechanism

Compile-time-known integer values can serve as type-level arguments.
Const-generic arguments (where a type parameter accepts a compile-time-known
value rather than a type) use this directly: `type Buffer[T, N: usize =
1024]: data: T[N]`. The full const-generic specification — value-kind
parameters, bounds, inference — is deferred to a future spec revision;
v1 supports the syntactic form shown above for fixed-size array sizing
and similar uses.

```
let arr: i32[fib(10) + 1]                  // valid; fib(10) + 1 is compile-time evaluable
type Buffer[T, N: usize = 1024]:
  data: T[N]
```

This is dependent-typing-lite: types can depend on compile-time-known values
without requiring full dependent type theory. The mechanism is uniform —
anything compile-time evaluable can appear in a type position requiring a
value.

#### 2.4.5 Negative literal parsing

A negative integer literal `-N` is parsed as a single signed
literal token for type-checking purposes. `let x: i8 = -5` checks the value
`-5` against `i8`'s range; it does not apply the runtime unary-minus operator
to a literal `5` (which would conflict with the rule that unary `-` on
unsigned integers is a type error — see §4.4.1). The runtime unary-minus
operator's rules still apply to runtime values; only literal parsing is
special.

#### 2.4.6 Reactivity provenance in diagnostics

The compiler tracks reactivity provenance through expressions. When an
expression's value is reactive, the compiler can identify the signal(s)
it depends on. This information surfaces in two places:

- **Errors that explicitly require compile-time evaluation.** A `const`
  declaration (§2.4.1.2) whose RHS is reactive is a compile error per
  §2.4.1.2's non-reactive guarantee. A type-level position requiring a
  compile-time-known value (e.g., a const-generic argument or an array
  length per §2.4.4) supplied with a reactive expression is likewise an
  error. The diagnostic names the source signal:

  ```
  const N: usize = sample_count(mouse_position)
  // error: `const` RHS must be non-reactive; value depends on signal
  //   `mouse_position` (at line 14). For a runtime-derived value, use
  //   `let` instead.
  ```

- **Diagnostic context, not error cause, for ordinary runtime bindings.**
  A reactive value assigned to a regularly-typed binding is *not* an error
  on reactivity grounds — `let x: u8 = compute(mouse_position)` is well-
  typed whenever `compute(mouse_position)` has type `u8` (or implicitly
  widens to `u8` per §4.5). Value-fits-type checking per §2.4.3 applies
  only to compile-time-known values; reactive values are checked by
  ordinary type-compatibility rules. If an error does arise (e.g., the
  reactive value's type doesn't match `u8` and no implicit widening
  applies), the diagnostic mentions the signal provenance to help the
  user trace the value's origin, but the underlying error is a type
  mismatch, not a fit-check failure.

#### 2.4.7 Implementation limits

Practical limits on compile-time evaluation (recursion depth, evaluation step
count, memory used) are implementation concerns. The compiler enforces
configurable limits to prevent runaway evaluation from hanging compilation.
When a limit is exceeded, the compiler reports an error indicating which limit
was reached and at what call site.

Floating-point compile-time evaluation uses the target's IEEE 754 format
exactly. Compile-time and runtime float computations on the same expression
must produce bit-identical results. This is a correctness requirement, not a
performance optimization.

---

## 3. Trait System

Traits are the language's abstraction mechanism for behavior. A trait declares
an interface — a set of method signatures, associated types, and requirements —
that types may explicitly conform to and provide implementations for. Generic
code expresses constraints in terms of traits; the compiler resolves trait
methods at monomorphization time per §2.3.5.

The trait system is nominal throughout: a type satisfies a trait only via an
explicit declaration of conformance, never by accidental structural match. Two
types with structurally identical method sets are distinct unless both have
explicitly declared (and implemented) the same trait.

### 3.1 Trait Declarations

A trait is declared with the `trait` keyword (grammar §3.7). The body of a
trait declares method signatures, associated types, requirements on other
traits, and optionally default values for the trait's defaulting behavior.

```
trait Display:
  fn display(value: Self) -> string

trait Add[Rhs = Self]:
  type Output = Self
  fn add(a: Self, b: Rhs) -> Output

trait Producer:
  type Item
  fn produce(value: Self) -> Option[Item]
```

A trait declaration may be empty:

```
trait Marker
```

Empty traits ("marker traits") have no methods, no associated types, and no
requirements. They serve as nominal tags — a type's `satisfies Marker` clause
is a declarative assertion the user makes about the type, checked only for
existence by the compiler.

#### 3.1.1 Method signatures

Trait methods are declared with the `fn` keyword inside the trait body. The
signatures use `Self` (capitalized, the type-level identifier) to refer to the
implementing type:

```
trait Eq:
  fn eq(a: Self, b: Self) -> bool
```

`Self` is a type-level placeholder bound during implementation: in a `fulfill
Eq for i32` block, `Self` resolves to `i32`, so the method's signature becomes
`fn eq(a: i32, b: i32) -> bool`.

Trait methods do not use a `self` parameter. The lowercase `self` keyword is
reserved exclusively for reactive context inside node and connection bodies
(§13). Trait method signatures name their receiver
parameter explicitly. The first parameter's type is conventionally `Self` for
methods that operate on instances, but trait methods may have any parameter
list — including no `Self` parameter at all (for "associated functions" like
constructors).

#### 3.1.2 Associated types

A trait may declare associated types using the `type` keyword inside the body:

```
trait Producer:
  type Item
  fn produce(p: Self) -> Option[Item]
```

`Item` is an associated type — a type-level name whose concrete value is bound
by each implementation. Associated types may be referenced in the trait's
method signatures and in other associated-type expressions.

An associated type may declare a default value:

```
trait Add[Rhs = Self]:
  type Output = Self
  fn add(a: Self, b: Rhs) -> Output
```

When an implementation does not bind `Output` explicitly, the default applies.

Implementations bind associated types via the `type Name = Concrete` form
inside `fulfill` blocks (§3.3.2).

Bounds on associated types in generic constraints use where-clauses with the
`.` member-access notation (§3.3.4 for where-clauses; §3.1.6 for generic
trait parameters):

```
fn sum[P: Producer](p: P) -> P.Item where P.Item: Numeric:
  ...
```

#### 3.1.3 Default method bodies

A trait may provide a default implementation for any of its methods by
including a function body in the trait declaration:

```
trait Greet:
  fn greet(value: Self) -> string

  fn shout(value: Self) -> string:
    greet(value).to_upper() + "!"
```

Here `greet` is abstract (no body, must be implemented); `shout` has a default
body that delegates to `greet`. An implementation may override the default by
providing its own body in the `fulfill` block, or accept the default by
omitting the method.

#### 3.1.4 Super-trait requirements (`requires`)

A trait may require that any type implementing it also implements other traits.
Requirements are declared with the `requires` keyword (grammar §3.7):

```
trait Student:
  requires Person
  fn enrollment_id(value: Self) -> string
```

A type `T` satisfies `Student` only if `T` also satisfies `Person`. The
compiler enforces this at the point `satisfies Student` is declared on the
type: if `Person` is not in the type's `satisfies` set (directly or
transitively), the declaration is rejected.

A child trait may not redeclare a method already declared by any of its
required traits (directly or transitively). If `Person` declares `fn display(
value: Self) -> string`, then `Student` declaring its own `fn display(value:
Self) -> string` is a compile error at the trait declaration site. The
reasoning: any type satisfying `Student` would also satisfy `Person` via
`requires`, so the type's effective method set would contain two `display`
methods — exactly the conflict §3.2.1 forbids. Rejecting redeclaration at the
trait level surfaces this problem at the trait author's site, not at the
type author's site.

This rule forecloses inheritance-style method override in trait hierarchies.
Child traits compose by adding *new* methods to the required trait's
interface, not by replacing existing ones. If a different behavior for an
existing method name is needed, the right tool is a separate trait (with a
different method name) or a newtype with its own conformance, not override
through `requires`.

The signature-no-redeclaration rule applies to signatures only. Default
bodies remain overridable at the *fulfill site* per §3.1.3 — a type
implementing the trait may provide its own body, replacing any default the
trait declared. The override happens at the type's implementation, not at
the trait level. This separation keeps signature stability (contracts don't
shift) decoupled from implementation flexibility (types choose how to fulfill
the contract).

Default bodies are themselves part of the trait that declares them. A child
trait via `requires` inherits the parent's signatures *and* their default
bodies; it cannot redeclare either. Overriding the default body happens only
at the fulfill site, not by re-providing a default in a child trait. This
preserves the principle that a method — both its signature and its default
body — has exactly one origin: the trait that originally declared it. Types
choose how to fulfill it; the trait hierarchy does not provide alternative
defaults at intermediate levels.

The `requires` mechanism is how trait hierarchies are constructed (§3.6).

#### 3.1.5 Trait-level default concrete type

A trait may declare a default concrete type used by the defaulting mechanism
(§3.6.2 for selection among multiple defaults; §4.9.3 for the numeric
default mappings). When a use site is constrained solely
by a trait (or traits) with declared defaults and nothing else pins the type,
the trait's default fires.

The default must itself satisfy the trait; this is compiler-enforced.

```
@default(i32)
trait Integer:
  requires Numeric, IntDiv, Rem, ...    // illustrative; canonical in §4.9.2
```

The exact syntactic form (annotation, dedicated keyword, body clause) is a
syntax detail; the semantic decision is that defaults are declared on the
trait, not on the compiler.

A trait without a declared default produces a compile error at any use site
that would require defaulting through it ("no default available for trait
X"). This treats missing defaults as a deliberate choice by the trait author:
some traits are too domain-specific to pick a default for.

Trait-declared defaults are the only defaulting mechanism in the language.
There are no compiler-internal defaults, no module-level pragmas, no
use-site overrides via alternative defaulting paths. When the default
mechanism does not fire (no constraining trait declares a default, multiple
incomparable defaults conflict per §3.6.2, or the user wants a non-default
type), the user resolves through explicit annotation, not through another
defaulting knob. This preserves the principle that defaults are discoverable
at the trait's declaration site and nowhere else.

#### 3.1.6 Generic traits

Traits may declare type parameters (grammar §3.7's `GenericParams`):

```
trait From[T]:
  fn from(value: T) -> Self

trait Add[Rhs = Self]:
  type Output = Self
  fn add(a: Self, b: Rhs) -> Output
```

Type parameters on a trait are part of the trait's identity at the
type-system level. Two terms are useful when discussing generic traits:

- A **trait instance** is the trait paired with specific concrete type
  arguments — e.g., `From[i32]` is one trait instance; `From[i64]` is a
  different trait instance. The type system treats each instance as a
  distinct constraint and a distinct dispatch target.
- A **parent trait identity** is the trait's declared name independent of
  generic arguments — for both `From[i32]` and `From[i64]`, the parent
  trait identity is `From`. Several conformance and dispatch rules
  (§3.2.1, §3.4.1.1) operate at parent-trait granularity.

A type may implement multiple trait instances of the same parent trait
(`fulfill From[i32] for MyNumber` and `fulfill From[i64] for MyNumber`
coexist; both share the parent `From`). Default type parameters (`Rhs =
Self`) follow the rules for generic parameters in §3.1.6 and §2.2.

##### 3.1.6.1 Default-type-parameter resolution

When a generic trait has defaulted type parameters (e.g., `trait
Add[Rhs = Self]`), references to the bare trait name resolve to the
trait instance with all defaults applied:

- In `requires` clauses: `requires Add` is sugar for `requires Add[Self]`.
- In trait bounds: `T: Add` is sugar for `T: Add[Self]`.
- In `satisfies` clauses: `satisfies Add` is sugar for `satisfies Add[Self]`.
- In `fulfill` blocks: `fulfill Add for T` is sugar for `fulfill Add[Self] for T`.
- In inferred constraints: the compiler infers `T: Add[Self]` unless the
  operation's operand types force a cross-type form.

This rule is universal across all generic traits with default type
parameters, not specific to operator traits. To reference a non-default
instance, the user supplies explicit type arguments: `Add[i64]`,
`From[string]`, etc.

The defaulting happens at name-resolution time; at code-generation
time, every reference has a fully-specified trait instance.

#### 3.1.7 Required attrs and consts (node and connection types only)

Traits may declare *required attrs* and *required consts* that
implementing types must provide. These requirements apply only to
node and connection types (see §13 for the reactive system); they
are not applicable to records, enums, newtypes, or primitive types.

```
trait Action:
  const type: string                    -- required const, no default
  attr enabled: bool = true              -- required attr with default

trait Identifiable:
  const id_kind: string = "generic"     -- required const with default
```

The forms `attr Name: Type [= Default]?` and `const Name: Type [=
Default]?` inside a trait body declare requirements. Defaults are
optional in trait declarations:

- **Without a default** — implementing types must supply the
  declaration with a concrete value.
- **With a default** — implementing types may omit the declaration
  (in which case the trait's default is used) or override it by
  declaring their own.

Override semantics parallel default method bodies (§3.1.3): the
trait declares the contract; the implementing type may accept the
default or override at its declaration site.

A node or connection type satisfies a trait with required attrs
and consts only if every required declaration is present (or
defaulted) in the type's body with a matching name and type:

```
trait Action:
  const type: string
  attr enabled: bool = true

node Log:
  satisfies Action
  const type: string = "@action/log"     -- supplies the no-default const
  -- enabled inherits the trait's default (true) automatically
  default attr message: string

node Delay:
  satisfies Action
  const type: string = "@action/delay"
  attr enabled: bool = false              -- overrides the trait's default
  default attr time: duration
```

Restrictions:

- Required attrs and consts are forbidden in traits implemented by
  records, enums, newtypes, or primitives — those types lack the
  reactive cell machinery. The compiler rejects `satisfies` on
  such types if the trait has required attrs or consts.
- Required reactive declarations (`signal`, `recurrent`, `derived`)
  inside a trait body are *not* supported in v1. Only `attr` and
  `const` requirements are recognized.
- The same name/type matching rules as method signatures apply: an
  implementing declaration must match the trait's required name
  and type exactly.

### 3.2 Conformance Declarations (`satisfies`)

A type declares conformance to a trait by including a `satisfies` clause in
its body (grammar §3.5 for records, grammar §3.6 for enums, grammar §3.8
for nodes, grammar §3.9 for connections):

```
type Person:
  satisfies Display, Hash
  first_name: string
  last_name: string
  age: i32
```

`satisfies` makes the conformance visible at the type's declaration site. A
reader of the type sees its full set of conformances without leaving the
type's file. The clause names the traits the type promises to implement; the
actual implementations live in `fulfill` blocks (§3.4), possibly in different
files (subject to the orphan rule from §3.7).

`satisfies` and `fulfill` are paired and both required for traits with
methods:

- `satisfies Trait` in a type body without a corresponding `fulfill Trait for
  Type` block reachable through the module graph is a compile error: the
  promise is unfulfilled.
- `fulfill Trait for Type` without a corresponding `satisfies Trait` in
  `Type`'s body is a compile error: the implementation has no declared
  contract.

The exception is traits with no methods (pure-requirement traits, §3.3.5):
these are automatically satisfied when all required traits are satisfied; no
`satisfies` clause is needed on the type and no `fulfill` block is needed for
the umbrella.

#### 3.2.1 No overlapping method names across satisfied traits

A type's `satisfies` set must not contain two *distinct trait identities*
whose method names overlap. If `Trait1` and `Trait2` (different traits, not
different instantiations of the same generic trait) each declare a method
named `display`, no type can declare `satisfies Trait1, Trait2` — the
compiler rejects the declaration with an error identifying the conflicting
method name and the two traits.

This rule preserves the contract semantics of `satisfies`. A reader of a
type's declaration sees the full set of contracts the type promises; if those
contracts had hidden naming conflicts, the contract sheet would be lying
about what `display` (or whichever method) does. By forbidding overlap at the
declaration site, the contract remains unambiguous: every method name on the
type maps to exactly one trait-method origin.

##### Generic trait instantiations do not conflict

Different generic instantiations of the *same parent trait* — e.g.,
`From[i32]` and `From[i64]`, or `Add[Self]` and `Add[Other]` — are
distinct trait instances per §3.1.6, but they share a parent trait
identity. Their method names refer to the same underlying trait method
parameterized over the trait's generic arguments. They do not conflict
under this rule:

```
type MyNumber:
  satisfies From[i32], From[i64]        // ✓ same parent trait From
  ...

fulfill From[i32] for MyNumber:
  fn from(value: i32) -> MyNumber: ...

fulfill From[i64] for MyNumber:
  fn from(value: i64) -> MyNumber: ...
```

The two `from` methods are disambiguated at call sites by argument type
(in the `From` direction, the source-value type pins the instance) or by
expected return type. Bare-name dispatch typically succeeds without
explicit annotation:

```
let n1 = MyNumber::from(5_i32)     // resolves to From[i32]::from
let n2 = MyNumber::from(5_i64)     // resolves to From[i64]::from
let n3: MyNumber = 5_i32.into()    // Into[MyNumber] from i32 — resolves through From[i32]
```

When inference cannot pick a unique instance (e.g., the argument is
polymorphic), explicit disambiguation via turbofish on the trait is
available per §3.4.1.1: `From::[i32]::from(value)`.

The conflict rule applies only to *different parent traits* with
overlapping method names. The universal identity `From[T] for T` (§7.3)
and a user-written `From[U] for T` are both instantiations of `From` and
therefore do not conflict — both are part of the same parent-trait
conformance.

##### Algorithm: effective method-set computation

Given a type `T` with `satisfies T1, T2, ..., Tn`, the compiler computes
`T`'s *effective method set* and checks for collisions:

1. Initialize the effective set as empty.
2. For each directly-satisfied trait `Ti`, compute the closure of `Ti` under
   the `requires` relation: `Ti` itself plus every trait reachable through
   any chain of `requires` clauses.
3. Union the method declarations of all traits in the closure for all `Ti`s
   into the effective set. Each entry is a (method-name, parent-trait-identity)
   pair, where *parent-trait identity* is the trait's declared name
   independent of generic arguments (so `From[i32]` and `From[i64]` share
   the parent identity `From`).
4. If two entries share the same method name but originate from different
   *parent-trait identities* (e.g., `Display`'s `display` and a different
   `MyDebug`'s `display`), the declaration is rejected. The error identifies
   the conflicting name and the two source parent traits.
5. Multiple entries with the same method name and the same parent-trait
   identity (i.e., different generic instantiations of the same parent —
   `From[i32]::from` and `From[i64]::from`) do *not* collide. They are
   logically the same method parameterized over generic arguments;
   dispatch among them is resolved by inference per §3.4.1.1.
6. Methods reached through multiple `requires` paths but originating from
   the same trait-method declaration are not in conflict — they are the
   same method, just reachable via multiple inheritance paths. This is the
   "diamond" case (well-defined in nominal trait systems) and is permitted.

The §3.1.4 rule (traits cannot redeclare methods from required traits)
guarantees that step 6's "same trait-method declaration reached multiple
ways" case has a single origin: the original declaring trait. There is no
ambiguity about which method is which when diamonds occur.

##### Workaround for legitimate dual conformance

When two traits a user wants both have conflicting method names *and
different parent identities*, the canonical workaround is the newtype
pattern: define separate newtype wrappers of the underlying type, each
satisfying one of the conflicting traits. Distinct newtypes have
distinct contract sheets and distinct method dispatches.

This workaround is unnecessary for different generic instantiations of
the same parent trait — those are permitted directly per step 5.

##### Consequence for dispatch

The rule shapes dispatch (§3.4): because no type can satisfy two traits
with *different parent identities* and overlapping method names, the
case of "multiple distinct-parent trait impls match this call site"
cannot arise. Call-site name resolution always finds at most one
parent-trait source for a given (type, method-name) pair. Within a
parent-trait source, multiple generic instantiations may match; these
are disambiguated by inference per §3.4.1.1.

### 3.3 Implementation Blocks (`fulfill`)

A `fulfill` block delivers a trait's implementation for a specific type:

```
fulfill Display for Person:
  fn display(value: Person) -> string:
    "{value.first_name} {value.last_name}"
```

The block lives in some module (subject to the orphan rule from §3.7), not
necessarily in the same module as either the trait or the type. Multiple
`fulfill` blocks for the same (trait, type) pair are rejected by the coherence
rule (§3.7): exactly one implementation exists per pair, reachable through
the module graph.

Functions defined inside a `fulfill Trait for Type` block live in a
*(Trait, Type)-scoped namespace*, not in the enclosing module's free-function
namespace. This is the key distinction from ordinary top-level function
definitions:

- A free function `fn display(p: Person)` defined at module level occupies a
  name slot in that module's free-function namespace. Per §10, function
  names are unique within their module; defining two free
  functions with the same name in the same module is a compile error.
- A function `fn display(value: Person)` defined inside `fulfill Display for
  Person` does *not* occupy the module's free-function namespace. It lives
  in the (`Display`, `Person`) trait-implementation namespace. The same
  module can contain multiple `fulfill` blocks for different (trait, type)
  pairs that each define functions named `display`; these do not conflict
  because they are in different namespaces.

This means stdlib (and user code) can define `fulfill Display for i32`,
`fulfill Display for i64`, `fulfill Display for f32`, etc. — all in the same
module — without name collisions, because each `display` is scoped to its
own (`Display`, `Type`) pair.

Coexistence with free functions: a module may simultaneously define a free
function `fn display(p: Person)` *and* contain a `fulfill Display for Person`
block whose method is also named `display`. The two functions live in
different namespaces and do not conflict at the definition site. They may
conflict at *call sites* under uniform-call-syntax dispatch — see §3.4 for
resolution rules.

The syntax (grammar addition):

```
FulfillItem  := 'fulfill' TypeExpr 'for' TypeExpr WhereClause? FulfillBody
FulfillBody  := NEWLINE INDENT FulfillBodyItem+ DEDENT
FulfillBodyItem := Annotation* DocComment? (FnDecl | AssocTypeBinding)
AssocTypeBinding := 'type' Ident '=' TypeExpr NEWLINE
```

`fulfill` is a reserved keyword.

#### 3.3.1 Method signatures and `Self` usage

`Self` is a type-level identifier that appears in trait declarations to refer
to the implementing type. Its use is asymmetric across declaration contexts:

- **In trait declarations**, `Self` is the standard way to refer to the
  implementing type, because the implementing type is not yet known. Trait
  authors write `fn display(value: Self) -> string`; there is no concrete
  name available to substitute, so `Self` is necessary.

- **In `fulfill` blocks**, the implementing type *is* known — it appears in
  the `for Type` portion of the `fulfill` declaration. The recommended form
  is to write the explicit type name in method signatures, not `Self`:

```
fulfill Eq for Person:
  fn eq(a: Person, b: Person) -> bool:
    a.first_name is b.first_name and a.last_name is b.last_name
```

`Self` remains *permitted* inside `fulfill` blocks and is treated as a
synonym for the implementing type (the compiler substitutes `Self` →
`Person` during type checking). The two forms produce identical signatures
and identical compiled code. The explicit-type-name form is preferred for
readability: a reader sees concrete types at every position, without an extra
indirection through `Self`.

Generic implementing types may make `Self` more convenient by keeping the
signature shorter:

```
fulfill Add for Vec3:
  type Output = Vec3
  fn add(left: Vec3, right: Vec3) -> Vec3:     // explicit
    Vec3(x: left.x + right.x, y: left.y + right.y, z: left.z + right.z)

fulfill Display for Result[T, E] where T: Display, E: Display:
  fn display(result: Result[T, E]) -> string:   // explicit, verbose but clear
    match result:
      Ok(value): "Ok({value.display()})"
      Err(error): "Err({error.display()})"
```

For generic types specifically, users may prefer `Self` to avoid repeating
the parameterization (`fn display(result: Self) -> string`). Both forms are
valid; the choice is stylistic.

The receiver parameter name (`a`, `value`, `result`, `left`, etc.) is always
the implementer's choice. There is no `self` keyword for trait method
receivers — that lowercase form is reserved exclusively for reactive context
inside node and connection bodies (§13). Explicit parameter naming is the
language's general principle under uniform function call syntax: every
parameter has a chosen name, not an implicit one.

Other type-level references in trait signatures (associated types like
`Output`, `Item`, etc.) follow the same substitution rule: in `fulfill`
blocks they may be written either with the trait's name (`Output`) or with
the concrete type bound to them.

#### 3.3.2 Associated type bindings

A `fulfill` block binds the trait's associated types via the `type Name =
Concrete` form:

```
fulfill Add for i32:
  type Output = i32
  fn add(left: i32, right: i32) -> i32:
    // built-in integer addition
```

Note: with the §4.9.1 default `type Output = Self`, the `type Output =
i32` binding shown here is explicit but optional. It could be omitted;
the default applies. Explicit binding is shown for clarity in examples
and remains valid where the implementer wants to make the choice
visible.

An associated type with a default value in the trait declaration may be
omitted in the `fulfill` block; the default applies. An associated type
without a default must be bound explicitly.

#### 3.3.3 Default-body overriding

When a trait declares a method with a default body (§3.1.3), the
implementation may either inherit the default by omitting the method or
override it by providing its own body:

```
trait Greet:
  fn greet(value: Self) -> string
  fn shout(value: Self) -> string:
    greet(value).to_upper() + "!"

fulfill Greet for Loud:
  fn greet(value: Loud) -> string:
    "HELLO"
  // shout inherited from trait default

fulfill Greet for Polite:
  fn greet(value: Polite) -> string:
    "hello"
  fn shout(value: Polite) -> string:
    "(politely): " + greet(value)
  // shout overridden
```

Abstract methods (no default body) must be implemented; their absence in a
`fulfill` block is a compile error.

#### 3.3.4 Conditional implementations (where clauses)

A `fulfill` block may be conditional on its type parameters satisfying
additional traits. The condition is expressed via a where-clause attached to
the `fulfill` declaration:

```
fulfill Display for Result[T, E] where T: Display, E: Display:
  fn display(result: Result[T, E]) -> string:
    match result:
      Ok(value): "Ok({value.display()})"
      Err(error): "Err({error.display()})"
```

The implementation is available only when the type parameters satisfy the
required traits. A `Result[i32, string]` implements `Display` because both
`i32` and `string` do; a `Result[ClosureType, string]` does not, because
closure types typically do not implement `Display`. The compiler verifies
the conditions at every call site that requires the implementation.

#### 3.3.5 Pure-requirement traits and automatic satisfaction

A trait that declares no methods and no associated types — only `requires`
clauses — is a pure-requirement trait. Examples are the umbrella traits from
§3.6 (`Numeric`, `Integer`, `Float`, `Signed`, `Unsigned`).

Pure-requirement traits are automatically satisfied when all required traits
are satisfied. No `fulfill` block is needed for the umbrella; no `satisfies`
clause is needed on the type for the umbrella (though it may be included for
documentation). The trait is *structurally* satisfied via the satisfaction of
its requirements, but it remains *nominally* present in the trait system:
generic constraints `T: Numeric` are checked against the trait's name, and
the compiler verifies that `T`'s satisfied trait set includes everything
`Numeric` requires.

This carves out the only point of structural satisfaction in the language's
otherwise-nominal trait system, and it is bounded: the structural rule
applies only to traits with no methods. Any trait with method signatures
requires explicit `satisfies` + `fulfill` per §3.2.

#### 3.3.6 Visibility of `fulfill` blocks

`fulfill` blocks have no visibility specifier of their own. An implementation
is visible wherever both the trait and the type are jointly visible. If a
caller can name `Display` (per its visibility) and can name `Person` (per
its visibility), the call resolves to the `fulfill Display for Person` block;
the implementation's visibility is the intersection of the trait's and type's
visibility scopes.

This avoids the case where a trait and type are both visible but the
implementation is not, which would produce a confusing "method not found"
error at a site where the method clearly should exist.

### 3.4 Trait Method Dispatch

The language uses uniform function call syntax: a function whose first
parameter is of type `T` is callable in three equivalent forms. Trait methods
participate in this uniformly. Given a `fulfill Display for Person` block
containing `fn display(value: Person) -> string`, any of the following are
valid calls (and equivalent):

```
person.display()              // method-call form
display(person)               // conventional form, requires `display` in scope
Display::display(person)      // trait-path form, no import needed
```

Operator application uses the `|>` token (§13.17); it is not a
function-call form. `>>` is bitwise right shift only. Functions are
called via the three forms above.

The trait-path form (`Trait::method`) requires no `use` import — the
path itself makes the trait accessible by path, satisfying the in-scope
requirement for dispatch (§3.4.1). The trait must still be visible at
the call site under §10's visibility rules; "no import needed" does not
override visibility. Per §3.2.1 the bare-name forms are never ambiguous between
traits with *different parent identities* (a type cannot satisfy two
traits with different parents and overlapping method names). When a type
satisfies multiple generic instantiations of the *same* parent trait
(e.g., `From[i32]` and `From[i64]`), bare-name dispatch is normally
disambiguated by inference from argument or expected return type per
§3.4.1.1; explicit disambiguation via the trait-path form
(`Trait::[T]::method`) is available when inference cannot select one.
The other forms rely on name resolution per §10.

#### 3.4.1 Resolution across free-function and trait-implementation namespaces

A bare-name call `f(x)` or method-call `x.f()` may
resolve to either a trait-implementation function or a free function. The
resolution algorithm prioritizes trait implementations over free functions:

1. **Trait-impl search.** For each trait `T` reachable in the current scope
   (imported or accessible by path) such that `x`'s type fulfills `T` and `T`
   declares a method named `f`, collect the trait-impl candidate
   `T::f(x, ...)`. The function bodies live inside the corresponding `fulfill
   T for X` blocks.
2. **Collapse candidates from the same parent trait.** Per §3.2.1, multiple
   candidates may arise when a type satisfies several generic instantiations
   of the same parent trait (e.g., `From[i32]` and `From[i64]` both
   declaring `from`). The compiler treats these as one logical method
   parameterized by the trait's generic arguments. Disambiguation among
   them uses the call-site context — argument types, expected return type,
   explicit turbofish — exactly as for any other generic function dispatch
   per §2.2.5. The set of candidates after this collapse contains at most
   one parent-trait entry.
3. **At most one parent-trait candidate after collapse.** Per §3.2.1, no
   type may satisfy two traits with *different* parent identities that
   declare overlapping method names — the type's `satisfies` declaration
   would have been rejected. Therefore the trait-impl search yields either
   zero or one parent-trait candidate.
4. **One parent-trait candidate matches → resolve to it.** The trait impl
   wins over any same-named free function. A free function with the same
   name in scope is *shadowed* at this call site; it remains callable only
   via path qualification (e.g., `some_module::f(x, ...)`). Within the
   parent-trait candidate, if multiple generic instantiations are
   reachable, the compiler resolves to a specific instantiation by
   inference per §2.2.5; failure to resolve to one is a compile error at
   the call site asking for explicit disambiguation.
5. **No trait-impl candidate matches → fall back to free-function search.**
   The compiler looks in the current scope's free-function namespace for a
   function `f` whose first parameter type matches `x`'s type (or is reachable
   via implicit widening per §4.5).
6. **One free function matches → resolve to it.** Standard free-function
   dispatch.
7. **Multiple free functions in scope under the same local name are
   impossible.** Free functions are uniquely named within their module per
   §10 (Option E in Topics.md); only one can be in scope under any
   given local name. Cross-module conflicts are resolved at import time, not
   at call time.
8. **Nothing matches → unknown method error.** The diagnostic includes the
   receiver's type, the unmatched name, and any near-matches the compiler
   identified.

The algorithm is deterministic: §3.2.1's parent-trait collision rule
guarantees that any (type, method-name) pair has at most one parent-trait
source, and the §10 module rules guarantee that any given module-scope
name has at most one free-function source. When a parent-trait source has
multiple generic instantiations, disambiguation among them follows the
standard inference rules (§2.2.5).

Trait-path syntax (`Trait::f(x)`) remains available as the explicit form
when a user wants to make the call's trait source visible at the call
site, including disambiguation between generic instantiations via
turbofish (`Trait::[T]::f(x)`, see §3.4.1.1 below) or via path-qualified
type-side dispatch (`Type::f(x)`, where `Type` is the for-type of the
target `fulfill` block).

#### 3.4.1.1 Disambiguating generic trait instantiations

When a type satisfies multiple instantiations of the same parent trait
(e.g., `MyNumber` satisfies both `From[i32]` and `From[i64]`), bare-name
dispatch at `MyNumber::from(value)` typically resolves via the argument
type: if `value: i32`, the `From[i32]` instantiation is selected; if
`value: i64`, the `From[i64]` instantiation. The compiler uses the same
inference algorithm as for generic functions (§2.2.5).

When inference cannot uniquely determine the argument's type — for
instance, inside a generic function body where the argument has a
generic-parameter type — the compiler reports a call-site ambiguity
error. The user disambiguates explicitly using the trait-path form with
turbofish on the trait:

```
fn build[T](v: T) -> MyNumber where MyNumber: From[T]:
  From::[T]::from(v)       // T is generic; turbofish pins the instantiation
```

This is the turbofish form (§2.2.5) applied to the trait identity,
selecting a specific instantiation of the trait before resolving the
method.

Trait visibility matters for dispatch. A `fulfill T for X` block is reachable
for dispatch on `x: X` only when `T` itself is in scope (imported or
accessible by path). If `T` is not in scope, the implementation is invisible
at the call site, and the search proceeds as if that trait-impl candidate
did not exist. Users control which trait-impl candidates participate in
dispatch by choosing which traits to import.

Disambiguation forms:

- `Trait::f(x, ...)` — explicit trait selection. While trait-vs-trait
  ambiguity cannot arise per step 3, the explicit form makes the trait
  source visible at the call site, which aids readability.
- `some_module::f(x, ...)` — explicitly select a free function (used when a
  free function would otherwise be shadowed by a trait impl per step 4).
- `x.f::[T]()` is *not* a disambiguation form; the turbofish (§2.2.5)
  specifies generic type arguments, not the receiving trait.

#### 3.4.2 Dispatch at monomorphization

Trait method calls in monomorphized code resolve to direct function calls per
§2.3.5; coherence (§3.7) guarantees there is exactly one implementation to
dispatch to within a (trait, type) pair. The free-function vs trait-impl
namespace distinction is purely for *name resolution at call sites* — once
resolved, the call compiles to a direct function call to a specific function
identified by its fully-qualified path (module-path-or-trait-path + name).

### 3.5 Argument Forms

The language supports two forms for supplying arguments at any call site:
*positional* and *named*. The choice is per-call, not per-callee, with one
universal restriction: positional and named arguments cannot be mixed
within a single call.

#### 3.5.1 The two forms

**Positional form** — arguments are listed in declaration order, without
names:

```
let s = Shape::Rectangle(10.0, 20.0)
let c = clamp(temperature, 0, 100)
```

**Named form** — each argument is paired with its parameter name:

```
let s = Shape::Rectangle(width: 10.0, height: 20.0)
let c = clamp(value: temperature, lower: 0, upper: 100)
```

The named form uses `name: value` syntax. In the named form, the order of
arguments does not matter; the compiler matches by name. In the positional
form, arguments must appear in declaration order.

Both forms are valid for any single-argument call. `square(5)` and
`square(value: 5)` are equivalent; no special rule restricts single-
argument calls to one form.

A no-argument call (`person.display()`) is trivially both forms; the
parentheses are empty and no mixing question arises.

#### 3.5.2 No mixing within one call

A single call site uses either positional or named form throughout. Mixing
is a compile error:

```
Shape::Rectangle(width: 10.0, 20.0)       // ✗ mixed — compile error
add(3, right: 4)                          // ✗ mixed — compile error
```

The rule applies to every call: free functions, trait methods, variant
constructors, and any other invocation. The compiler reports the error at
the call site, identifying which argument breaks the pattern.

#### 3.5.3 Per-callable form constraints

Some declarations restrict the allowed form at their call sites:

- **Records** (§6.1.3) are *always* constructed with named arguments.
  Positional construction of records is a compile error.
- **Tuples** (§9) are *always* constructed positionally. Named
  construction of tuples is a compile error.
- **Newtypes** (§6.3.2) are *always* constructed positionally with one
  argument — the underlying value.
- **Free functions and trait methods** accept either form per-call.
  Parameters always have names (per §3.1.1), so both forms are available
  at every call site.
- **Enum variants** depend on the variant's declaration (§6.2.1):
  positionally-declared variants accept only positional form;
  named-declared variants accept both forms per-call.

The constraints reflect the nature of each declaration:

- *Records* are nominal product types whose fields are named for domain
  meaning. Forcing named construction makes the meaning of each value
  explicit at every construction site and prevents the
  same-typed-fields-in-wrong-order class of bugs (`Point(1.0, 2.0)` —
  which is `x` and which is `y`?). The verbosity is the cost; clarity is
  the benefit.
- *Tuples* are anonymous products whose fields have only positional
  identity — they have no names by design. Forcing positional
  construction preserves this anonymity; named construction would invent
  metadata that doesn't exist in the type.
- *Newtypes* wrap a single underlying value. The constructor takes one
  argument; the name would be redundant with the type name itself.
- *Enum variants* choose their available forms at the declaration site
  (§6.2.1). A positional declaration (`Some(T)`) commits to
  conciseness; a named declaration (`Rectangle(width: f64, height: f64)`)
  enables both forms for readability at call sites where names help.

For declarations that accept both forms, the choice between positional
and named at a call site is a style decision driven by readability. Long
argument lists, arguments with non-obvious meaning, or arguments using
defaults benefit from named form; short calls with self-evident argument
meaning benefit from positional form.

#### 3.5.4 Defaults and form interaction

Parameters with default values (per §6.1.2 for records and analogous
features for functions) may appear in any position in the parameter
list, including before non-defaulted parameters. Call sites resolve as
follows:

- In **named form**, default-bearing parameters may be omitted; the
  default value applies for any parameter not named in the call.
  Non-defaulted parameters must still be supplied.
- In **positional form**, parameters must be supplied in declaration
  order. A defaulted parameter mid-list cannot be skipped using
  positional form alone — every subsequent positional argument must be
  supplied as well, which means non-defaulted parameters following
  defaulted ones force the defaulted ones to also be supplied
  positionally. To skip a mid-list default, use named form.
- **Mixed form** (positional then named) is still forbidden per §3.5.2.

The relaxation (defaulted-before-non-defaulted permitted) is uniform
across functions, operators (§13.17.3), and constructor invocations.
The rule rewards named-argument call sites for readability.

```
fn greet(name: string, greeting: string = "Hello", suffix: string = "!"):
  ...

greet("Alice")                                  // ✓ uses both defaults
greet("Alice", "Hi")                            // ✓ uses suffix default
greet("Alice", "Hi", "?")                       // ✓ all positional
greet(name: "Alice", suffix: "?")               // ✓ named, skipping greeting
greet("Alice", suffix: "?")                     // ✗ mixed positional and named
```

Defaulted parameters may precede non-defaulted ones:

```
fn render(scale: f32 = 1.0, content: string):
  ...

render(content: "hello")                        // ✓ named, uses scale default
render(scale: 2.0, content: "hello")            // ✓ named, both supplied
render(1.0, "hello")                            // ✓ positional, both supplied
render("hello")                                 // ✗ positional cannot skip
                                                //   scale to bind content
```

The skipping flexibility of named form is one of its principal practical
advantages. Functions with many optional parameters typically benefit
from named form at call sites.

#### 3.5.5 Method calls and the receiver

A method call `x.f(args)` always passes the receiver `x` positionally
(per §3.4's uniform call syntax — the method call is sugar for `f(x,
args)`). The argument form rule applies to `args`, not to the receiver:

```
person.display()                                  // no args; trivially valid
shape.set_dimensions(width: 10.0, height: 20.0)   // named form for trailing args
shape.set_dimensions(10.0, 20.0)                  // positional form
shape.set_dimensions(10.0, height: 20.0)          // ✗ mixed
```

The receiver `x` is conceptually the first positional argument of the
underlying free function; the dot-syntax just brings it forward
syntactically.

#### 3.5.6 The `with` expression uses named form

The record-update `with` expression (§6.1.5) uses named form for its
field overrides:

```
let p2 = p1 with name: "new", age: 30
```

This is a special case of the general rule: records require named form
(§3.5.3); the `with` expression updates record fields and therefore
inherits the same form requirement. There is no positional `with` form.

#### 3.5.7 Argument forms in patterns

The same positional/named distinction applies to *patterns* that
destructure compound values (§6.2.4). Variant patterns may be positional
or named, parallel to variant construction; mixing within one pattern is
a compile error. Record patterns are always named; tuple patterns are
always positional.

This parallelism is structural: a pattern is a "call site for
destructuring," with the same argument-form rules as a call site for
construction.

### 3.6 Trait Hierarchies

Traits compose into hierarchies via `requires` clauses. The recommended
pattern, used pervasively in the language's standard library, is *fine-grained
operator/capability traits combined into umbrella traits*.

#### 3.6.1 The fine-grained-plus-umbrella pattern

Fine-grained traits each declare exactly one method or one closely related
group of methods, defining a single capability:

```
trait Add[Rhs = Self]:
  type Output = Self
  fn add(a: Self, b: Rhs) -> Output

trait Sub[Rhs = Self]:
  type Output = Self
  fn sub(a: Self, b: Rhs) -> Output

trait Mul[Rhs = Self]:
  type Output = Self
  fn mul(a: Self, b: Rhs) -> Output

trait Neg:
  fn neg(value: Self) -> Self
```

Umbrella traits combine fine-grained traits via `requires` clauses,
introducing no new methods. The numeric umbrellas follow this pattern;
canonical definitions appear in §4.9.2, abbreviated here as an
illustration:

```
@default(i32)
trait Numeric:
  requires Add, Sub, Mul, Zero, One, ...      // canonical: §4.9.2

@default(i32)
trait Integer:
  requires Numeric, IntDiv, Rem, ...          // canonical: §4.9.2

@default(f64)
trait Float:
  requires Numeric, Neg, Div, ...             // canonical: §4.9.2

@default(i32)
trait Signed:
  requires Integer, Neg, ...                  // canonical: §4.9.2

@default(u32)
trait Unsigned:
  requires Integer                            // Neg deliberately absent (§4.9.2)
```

The signed/unsigned split is structurally honest: `Neg` lives on `Signed`
and `Float`, not on `Numeric`. Unsigned integer types satisfy `Numeric`
and `Unsigned` but not `Neg`; this is what the umbrella's `requires` set
encodes. See §4.9.2 for the full umbrella definitions.

Per §3.3.5, umbrella traits are automatically satisfied when their
requirements are. Users implement the fine-grained traits for their types;
umbrella satisfaction follows.

Some fine-grained traits are deliberately *not* part of any numeric umbrella
because they are not numeric-specific. `Ord` (ordering) and `Eq` (equality)
are standalone fine-grained traits — non-numeric types (strings, enums,
records, user-defined types) may also be ordered or compared, so binding
`Ord` and `Eq` to the numeric hierarchy would either incorrectly require
non-numeric types to be numeric or fragment the standalone traits into
numeric and non-numeric versions. The clean answer: `Ord` and `Eq` stand on
their own; built-in numeric types implement both; the numeric umbrella traits
do not require them. A generic function needing both ordering and arithmetic
constrains as `T: Numeric & Ord`, combining the umbrella with the standalone
trait explicitly.

This pattern serves three purposes:

- *Precision in inferred constraints (§2.2.4):* the compiler infers exactly
  which fine-grained traits a function body requires, not a coarser umbrella
  the function might not actually need.
- *Convenience in explicit constraints:* users writing explicit bounds can use
  umbrella names (`T: Numeric`) without spelling out every operator, while
  still being able to write fine-grained bounds (`T: Add + Mul`) when
  precision matters.
- *A place for trait-level defaults:* umbrellas are the natural carrier of
  defaulting policy (§3.1.5), because the default is a property of the
  domain-level abstraction, not of any individual operator.

#### 3.6.2 Default trait selection in defaulting

When a use site is constrained by multiple traits each with declared defaults,
the most-specific trait in the hierarchy wins. "Most specific" is defined by
the `requires` relation: trait `A` is more specific than trait `B` if `A`
transitively requires `B` (i.e., `A` is "below" `B` in the hierarchy).

For example, a use site constrained by `Float` defaults to `f64` (the
`Float` trait's declared default), not `i32` (the `Numeric` trait's default),
because `Float` requires `Numeric` and is therefore more specific.

When multiple incomparable traits are in scope (neither requires the other)
and each has a declared default, the defaulting is ambiguous and the compiler
reports an error requiring an explicit annotation at the use site.

### 3.7 Coherence and Orphan Rules

Coherence is the property that for every (trait, type) pair, exactly one
implementation exists, reachable through the module graph. The language
enforces coherence structurally via the orphan rule.

#### 3.7.1 The strict orphan rule

A `fulfill Trait for Type` block is permitted in module M if and only if:

- `Trait` is defined in M, OR
- `Type` is defined in M.

A `fulfill` block where both `Trait` and `Type` are foreign to M is rejected
at compile time. This guarantees that no two independent modules can write
conflicting implementations for the same (trait, type) pair: at least one of
them would violate the orphan rule.

There are no exceptions for "private" or "non-exported" orphan implementations.
The privacy boundary cannot be enforced cleanly under separate compilation,
and the looser rules used in some languages produce confusing visibility
interactions. The strict rule is the only model that composes cleanly with
the language's separate compilation model (§2.3.2) and uniform call dispatch
(§3.4).

#### 3.7.2 Generic-parameter coverage

For impls involving type parameters, the orphan rule applies to the head of
the type expression: at least one *concrete local type* must appear in the
trait-or-type part of the `fulfill` declaration.

```
// Permitted in module M defining LocalType:
fulfill ForeignTrait[LocalType] for ForeignType: ...

// Rejected — no concrete local type:
fulfill ForeignTrait[T] for ForeignType: ...
```

The covering rule prevents two independent modules from each writing
`fulfill ForeignTrait[T] for ForeignType` with different unspecified `T`,
which would create conflicts at use sites.

#### 3.7.3 Language-privileged implementations

Certain implementations are provided by the language itself rather than by
user modules, and are not subject to the orphan rule:

- *Auto-implementations of built-in numeric traits for built-in numeric
  types.* The fine-grained operator traits (`Add`, `Sub`, `Mul`, etc.) are
  pre-implemented for the built-in numeric types. User code cannot redefine
  these.
- *Auto-derivations from `From` to `Into` and `TryFrom` to `TryInto`* (§7).
  When a user writes `fulfill From[T] for U`, the
  language automatically provides `Into[U] for T`. The derivation is built
  in, not user-writable.
- *Identity conversion `From[T] for T` for every type.* Universally provided.
- *Auto-implementations of `Copy` for built-in types per §11.4.1.* The
  primitive numeric types, `bool`, `char`, `string`, `duration`, `instant`,
  tuples of Copy components, and `Range[T]` when T: Copy all auto-implement
  Copy. User code cannot redefine these impls.
- *Stdlib auto-impl `From[()] for Option[T]`* — provides the `None` value
  for use in the `?` desugaring per §8.4.1. The impl is universally
  available for any T; user code cannot override or shadow it.
- *Stdlib-privileged borrow-returning functions* (§11.9.5 carve-out).
  Specific stdlib functions (e.g., indexed-collection accessors like
  `element_at`) declare return types of the form `&T`. The carve-out is
  enumerated in stdlib documentation; user-defined functions cannot
  declare borrow return types.

These privileged implementations exist outside the user-writable
`fulfill`-block space and cannot conflict with user code.

#### 3.7.4 Language-defined marker traits

A small, closed category of traits are declared by the language itself,
have no methods and no associated types, and receive compiler-privileged
enforcement. A type opts into one of these traits via the usual
`satisfies` clause or via `@derive` (every member of the category is
`@derive`-eligible; see §3.8.1); the compiler
treats membership as a flag carrying load-bearing semantics rather than
as a vehicle for user-supplied method bodies. Members of this category
are not redeclarable: user code cannot define a new trait of the same
name and reuse the privileged behavior.

The members of the category in Ductus v1 are:

- `Copy` (§11.4) — flags a type whose values are duplicated implicitly
  at every use site. The compiler enforces the auto-derivation rules
  and the use-site duplication semantics.
- `Circularity` (§13.6.5) — flags a connection type that may participate
  in topology cycles in the node graph. The compiler enforces the static
  cycle rule against this flag.

Like any trait, a language-defined marker trait may be used as a generic
bound or appear in a `where` clause:

```
fn duplicate[T: Copy](value: T) -> (T, T):
  (value, value)
```

This category is distinct from two superficially similar things:

- `Drop` (§14.8) is compiler-aware but carries a method (`fn drop`); it
  is therefore not a marker trait.
- The empty `trait Marker` shown in §3.1 illustrates a *user-writable*
  pattern — empty traits whose only purpose is to act as a nominal tag.
  Such user-defined empty traits are perfectly valid, but they are not
  members of the language-defined marker traits category and do not
  receive any compiler privilege beyond the usual `satisfies` check.

Adding a new member to this category is a language-level change, not
a user-extensible mechanism.

#### 3.7.5 Newtype pattern as orphan-rule workaround

When a user wants to implement a foreign trait for a foreign type, the
canonical workaround is the newtype pattern: wrap the foreign type in a local
newtype, then implement the foreign trait for the local newtype:

```
type MyVec[T]:
  wraps Vec[T]
  satisfies SomeForeignTrait

fulfill SomeForeignTrait for MyVec[T]:
  ...
```

`MyVec` is local to the user's module; the orphan rule is satisfied.
Newtype semantics are specified in §6.3.

### 3.8 Automatic Derivation (`@derive`)

For a fixed set of common traits, the language provides automatic structural
derivation via the `@derive` annotation (grammar §3.3). Applying `@derive` to
a type generates the appropriate `fulfill` blocks structurally, saving the
user from writing mechanical implementations.

#### 3.8.1 Derivable traits

The traits eligible for automatic derivation are:

- `Eq` — structural equality.
- `Ord` — structural total ordering.
- `Hash` — structural hashing.
- `Clone` — deep structural copy.
- `Display` — default human-readable formatting.
- `Debug` — default debug formatting (structural, compiler-defined).
- `Copy` (§3.7.4) — language-defined marker trait; derivation performs the
  structural Copy-eligibility check (every field's type must be `Copy`),
  no method body is generated.
- `Circularity` (§3.7.4) — language-defined marker trait; derivation is the
  opt-in declaration, no method body is generated.
- Any other language-defined marker trait (§3.7.4). The general rule:
  every member of the language-defined marker traits category is
  `@derive`-eligible. Derivation performs whatever structural check the
  marker's category requires (none for Circularity; Copy-eligibility for
  Copy) and emits the satisfies-flag with no method body.

The set is fixed in the language: the six structural-derivation traits
above (Eq, Ord, Hash, Clone, Display, Debug) plus every member of the
language-defined marker traits category (§3.7.4). Users cannot register
new traits for `@derive`. Other traits require manual `fulfill` blocks.
(A future extension may add user-definable derivation; not in v1.)

#### 3.8.2 Structural derivation rules

For a record type, derivation operates field-by-field:

- `@derive(Eq)` generates an implementation that compares each field pairwise
  using that field type's own `Eq` implementation.
- `@derive(Ord)` generates lexicographic ordering by field declaration order.
- `@derive(Hash)` generates a hash combining each field's hash.
- `@derive(Clone)` generates a structural copy of each field.
- `@derive(Display)` generates a default record-formatted string (exact format
  is compiler-defined).
- `@derive(Debug)` generates a structural debug format.

For an enum type, derivation operates variant-by-variant:

- `@derive(Eq)` generates an implementation that compares variant tags and,
  for matching tags, compares payload fields pairwise.
- `@derive(Ord)` orders by variant declaration order, breaking ties by
  payload comparison.
- The other derivations follow the same structural pattern.

For a newtype (§6.3), `@derive` may delegate to the underlying type
or operate structurally over fields, depending on the newtype's shape; see
the newtype section for details.

For language-defined marker traits (`Copy`, `Circularity`, and any future
markers per §3.7.4): derivation performs the marker's structural check
(Copy-eligibility for `Copy`; none for `Circularity`) and emits the
satisfies-flag — no method body is generated. The marker-trait derivation
is purely a structural opt-in.

Derivation requires every field's (or payload's) type to itself satisfy the
trait being derived. `@derive(Eq)` on `type Foo: x: SomeType` requires
`SomeType: Eq`. If any component type does not satisfy the trait, derivation
fails with a compile error identifying the offending component.

#### 3.8.3 Overriding derived implementations

A type may both `@derive` a trait and provide a manual `fulfill` block for
the same trait. The manual `fulfill` block takes precedence; the derived
implementation is suppressed for that (trait, type) pair.

This allows users to start with derived defaults and override specific
implementations as needed without removing the `@derive` annotation.

### 3.9 Custom Literal Suffixes (`@literal_suffix`)

The `@literal_suffix` annotation registers a typed-literal suffix for a
type. After registration, the lexer recognizes `<NumericLiteral><suffix>`
as a single token, and the type checker resolves it to a call of the
registered constructor function.

```
@literal_suffix("hz",    from_hz)
@literal_suffix("khz",   from_khz)
@literal_suffix("cents", from_cents)
type Frequency:
  wraps i64

fn from_hz[N: Numeric](n: N) -> Frequency:
  Frequency(n as i64)
fn from_khz[N: Numeric](n: N) -> Frequency:
  Frequency((n as f64 * 1000.0) as i64)
fn from_cents[N: Numeric](n: N) -> Frequency:
  ...

let middle_c = 440hz       -- resolved to from_hz(440), i64 literal
let voice    = 1.5khz      -- resolved to from_khz(1.5), f64 literal
```

Note: the registered type must be a distinct nominal type for the
suffix to provide type-level distinction. A bare type alias (`type
Frequency = i64`) is interchangeable with its underlying type and
defeats the purpose; use a newtype (§6.3) to create a fresh nominal
type.

#### 3.9.1 Annotation grammar

```
@literal_suffix("<suffix>", <constructor>)
```

- `"<suffix>"` is a string literal naming the suffix. Suffixes consist of
  one or more identifier-continue characters (letters, digits, underscores,
  Unicode identifier characters). Examples: `ms`, `khz`, `μs`, `years_2k`.
- `<constructor>` is the unqualified name of a function (or trait method)
  in scope, which must take exactly one numeric parameter (`Numeric`-bound)
  and return the annotated type.

Multiple `@literal_suffix` annotations may decorate one type, registering
distinct suffixes. Each (suffix, target-type) pair must be unique within
its scope; duplicate registrations are compile errors. Each *suffix*
must also resolve to exactly one (suffix, target-type) pair at any use
site; cross-module collisions where two modules register the same
suffix for different types are compile errors at the use site that
imports both. There is no qualified-suffix disambiguation syntax in v1;
users must avoid suffix-name collisions across imported modules.

The reserved suffix set (forbidden as `@literal_suffix` names) consists
of all built-in numeric type names — `i8`, `i16`, `i32`, `i64`, `i128`,
`u8`, `u16`, `u32`, `u64`, `u128`, `isize`, `usize`, `f32`, `f64` —
plus the built-in `duration` suffixes (§9.4.1.1). Registering any of
these as a `@literal_suffix` is a compile error. The numeric type
names are reserved to prevent confusion with the underscore-separated
numeric type suffix form (`5_i32`); the duration suffixes are reserved
because they are built into the language.

#### 3.9.2 Constructor signature

The constructor must:

- Take exactly one parameter, either of a concrete numeric type or
  bounded by `Numeric` (or one of its sub-traits). Generic
  constructors allow a single registration to handle both integer
  and float literals at the suffix.
- Return the annotated type.
- Be pure (no side effects, no reactive cell reads).

Each (suffix, target-type) pair has exactly one registered constructor.
Overloading the same suffix with multiple non-generic constructors for
the same target type (e.g., one taking `i64` and another taking `f64`)
is a compile error; use a single generic constructor instead.

```
-- Recommended: single generic constructor handles both forms
@literal_suffix("hz", from_hz)
fn from_hz[N: Numeric](n: N) -> Frequency: ...

-- Disallowed: two registrations for the same (suffix, target)
@literal_suffix("hz", from_hz_int)     -- compile error: duplicate registration
@literal_suffix("hz", from_hz_float)
```

#### 3.9.3 Resolution

When the lexer encounters `<NumberLiteral><suffix>`:

1. The lexer emits a single suffixed-literal token.
2. The type checker looks up `(suffix)` in the registered annotations
   visible at the site. A suffix must resolve to exactly one constructor
   in scope; ambiguity or no-match is a compile error.
3. The literal value is passed to the constructor; the call's return value
   is the literal's value.

The resolution happens at compile time; no runtime dispatch is involved.

#### 3.9.4 Built-in suffixes

The `duration` type (§9.4) has built-in suffixes:
`ns`, `us`, `μs`, `ms`, `s`, `min`, `h`, `d`. These are reserved and may
not be re-registered for another type in the same scope.

#### 3.9.5 Scope and visibility

`@literal_suffix` registrations follow normal name-visibility rules:
registrations made in a module are visible within that module and to
importers per §10. Built-in suffixes (for `duration`) are globally visible.

---

## 4. Numeric System

This section specifies the language's numeric primitive types, literal forms,
operator semantics, conversion rules, overflow handling, and the trait
hierarchy that underpins generic numeric code. The trait machinery in §3
provides the abstraction layer; this section provides the concrete numeric
content.

### 4.1 Numeric Primitive Types

The built-in numeric primitive type set is fixed at fourteen types.

**Signed integers:**

| Type    | Width                  | Range              |
|---------|------------------------|--------------------|
| `i8`    | 8-bit                  | −128 to 127        |
| `i16`   | 16-bit                 | −32,768 to 32,767  |
| `i32`   | 32-bit                 | −2³¹ to 2³¹ − 1    |
| `i64`   | 64-bit                 | −2⁶³ to 2⁶³ − 1    |
| `i128`  | 128-bit                | −2¹²⁷ to 2¹²⁷ − 1  |
| `isize` | platform-pointer-sized | platform-dependent |

**Unsigned integers:**

| Type    | Width                  | Range              |
|---------|------------------------|--------------------|
| `u8`    | 8-bit                  | 0 to 255           |
| `u16`   | 16-bit                 | 0 to 65,535        |
| `u32`   | 32-bit                 | 0 to 2³² − 1       |
| `u64`   | 64-bit                 | 0 to 2⁶⁴ − 1       |
| `u128`  | 128-bit                | 0 to 2¹²⁸ − 1      |
| `usize` | platform-pointer-sized | platform-dependent |

**Floating-point:**

| Type  | Format          | Range/Precision                  |
|-------|-----------------|----------------------------------|
| `f32` | IEEE 754 single | ~7 decimal digits, ±3.4 × 10³⁸   |
| `f64` | IEEE 754 double | ~16 decimal digits, ±1.8 × 10³⁰⁸ |

`i128` and `u128` are first-class. The performance overhead on platforms
without native 128-bit hardware is bounded (software emulation, ~3–5× a
64-bit op) and paid only when used; the alternatives — standard-library
big-integer types or manual u64 pairs — are dramatically worse ergonomically
for the legitimate use cases (UUIDs, cryptography, high-precision fixed-point,
financial domains beyond 64-bit range).

`isize` and `usize` are platform-sized integers. They are distinct types
serving distinct roles: `isize` is the array-length and index type
(§9.3); `usize` exists for FFI compatibility with C-family `size_t`
APIs, byte-count contexts where the non-negative invariant is load-bearing,
and low-level memory layout work. Most code uses `isize`; `usize` appears in
low-level corners.

Half-precision (`f16`) and quadruple-precision (`f128`) floats are not
included in v1. Hardware support for `f16` remains uneven across target
platforms, and many of the special numeric operations (§4.6) would require
fallback through `f32`. `f128` is highly specialized; the rare cases
needing it are adequately served by standard-library arbitrary-precision
types.

### 4.2 Type Aliases

The standard library provides convenience aliases using the `alias type`
mechanism (Topic 18 in the decision log; see §6.3 for newtypes, which
contrast with `alias type`):

```
alias type byte = u8
alias type short = i16
alias type int = i32
alias type long = i64
alias type double = f64
```

These are true aliases — transparent substitution, shared identity, fully
interchangeable with the underlying type at every use site. A value of type
`int` *is* a value of type `i32`; the alias adds no new identity, no new
trait impls, no conversion cost. Aliases are provided for users who prefer
C-traditional names; the canonical explicit-width names remain the
language's primary identifiers and appear unaltered throughout the standard
library and documentation.

No alias for `f32` is provided. The natural C-traditional name `float` would
conflict with the lowercase `float` placeholder keyword (§1.4 and §2.2.4)
and would mislead users from C-family languages
where `float` is single-precision. `double` is the canonical short name for
`f64`; users wanting `f32` write `f32` directly.

No alias is provided for `i128`, `u128`, `isize`, `usize`, `i8`, `u16`,
`u32`, or `u64` — these types have no widely-shared traditional short name
across language families, and the explicit-width names are clearer than any
alias would be.

Users may define their own aliases anywhere using the same `alias type`
syntax. The built-in aliases are a stdlib convenience; nothing about them is
privileged.

### 4.3 Numeric Literals

Literal forms follow grammar §2.5.

#### 4.3.1 Integer literals

Integer literals are written in decimal, hexadecimal (`0x` prefix), octal
(`0o` prefix), or binary (`0b` prefix). Underscores are permitted between
digits as visual separators:

```
42
1_000_000
0xFF_FF
0b1010_1100
0o755
```

An integer literal may carry an explicit type suffix, separated by an
underscore:

```
5_i32
255_u8
1_000_000_u32
0xFF_u8
```

The suffix forces the literal to the specified type, bypassing both the
placeholder mechanism (§2.1) and the trait-level default (§3.1.5). The
value-fits check from §2.4.3 still applies: a suffix specifying a type the
value doesn't fit (`300_u8`) is a compile error.

Without a suffix, an integer literal produces a value with the *integer
placeholder*. Resolution proceeds per §2.1 (use-site resolution, with
cross-kind permitted when the value fits exactly per §2.4.3).

#### 4.3.2 Float literals

Float literals require at least one of: a decimal point with digits, an
exponent (`e` or `E`), or an explicit float suffix:

```
3.14
1.0
1e6
2.5e-3
6.022_e23
```

A bare `1` is an integer literal; `1.0`, `1e5`, `1_f32` are float literals.
The grammar requires a digit on each side of the decimal point — leading-dot
forms like `.5` are not permitted; write `0.5`.

Float literals may carry suffixes:

```
3.14_f32
3.14_f64
1.0_f32
6.022e23_f64
```

Without a suffix, a float literal produces a value with the *float
placeholder*. Resolution proceeds per §2.1; the default per §3.1.5 is `f64`.

#### 4.3.3 Suffixed-literal forms for non-numeric types

In addition to the underscore-separated numeric type suffix
(`5_i32`, `3.14_f32`), a numeric literal may carry an
*identifier suffix* (no underscore separator) that produces a value
of a non-numeric type. The lexer recognizes
`<NumberLiteral><identifier>` as a single suffixed-literal token; the
type checker resolves the suffix against the language's built-in
suffixes and any user-registered suffixes (§3.9).

Built-in suffixed-literal forms in the language:

- `duration` suffixes (§9.4.1.1): `ns`, `us`, `μs`, `ms`, `s`, `min`,
  `h`, `d`. Both integer and float literals accept these.

Examples:

```
500ns         -- duration: 500 nanoseconds
100ms         -- duration: 100 milliseconds
1.5s          -- duration: 1.5 seconds (float)
2h            -- duration: 2 hours
```

User-defined suffixes via `@literal_suffix` (§3.9) follow the same
lexical rule and resolve via the registered constructor function.

The lexer distinguishes the underscore-separated type suffix from the
identifier suffix by the presence of the underscore: `5_i32` is the
former (forced numeric type); `5ms` is the latter (suffixed literal).

#### 4.3.4 Boolean and character literals

`true` and `false` are the two values of `bool` (§9.1.1). They
are not numeric; they do not participate in the numeric trait hierarchy.

Character literals (`'a'`) produce values of type `char` (32-bit Unicode
scalar value); byte literals (`b'a'`) produce values of type `u8`. Per
§9.1.2, `char` is not numeric; `u8` from a byte literal is
fully numeric (it is u8 in every type-system sense).

### 4.4 Operator Semantics

Operators on numeric values follow the rules in this section. Each operator
corresponds to one or more trait methods in §4.9's trait hierarchy.

#### 4.4.1 Arithmetic operators

| Operator     | Operand Constraint | Result                                                            | Notes                               |
|--------------|--------------------|-------------------------------------------------------------------|-------------------------------------|
| `+`          | `Add`              | Output (associated type)                                          | mixed-kind promotes per §4.5        |
| `-` (binary) | `Sub`              | Output (associated type)                                          | mixed-kind promotes per §4.5        |
| `*`          | `Mul`              | Output (associated type)                                          | mixed-kind promotes per §4.5        |
| `/`          | `Numeric`          | Self (on Float umbrella; mixed-kind operands widen per §4.4.1.1)  | mathematical division; see §4.4.1.1 |
| `//`         | `IntDiv`           | Output (associated type)                                          | truncating integer division         |
| `%`          | `Rem`              | Output (associated type)                                          | mixed-kind promotes per §4.5        |
| `-` (unary)  | `Neg`              | same as operand                                                   | type error on unsigned              |

##### 4.4.1.1 The `/` operator and integer-to-float promotion

The `/` operator is mathematical division, divorced from machine
representation. It accepts `Numeric` operands (integer, float, or mixed)
and always produces a `Float` result. `5 / 2` produces `2.5`, not `2`. The
result type is determined by the operator itself, not by the operand types:
even uniformly-integer operands (`10_i32 / 5_i32`) produce a `Float`, not
an integer.

The mechanism is a language-level rule applied at the operator, distinct
from direct trait dispatch:

1. The compiler verifies both operands satisfy `Numeric` (per §3.6).
2. If either operand is `Integer`-kinded (or both are), the compiler
   inserts implicit widening conversions to lift them to the appropriate
   `Float` type per §4.5's lossless-widening rules. The pragmatic
   exception for `i64`/`u64` → `f64` (§4.5.2) applies here too.
3. The promoted operands then satisfy `Div` (which is declared only on
   `Float`); the compiler dispatches `Div::div` on the float type.
4. The result is a `Float` value of the type the operands were widened to.

Examples:

```
5_i32 / 2_i32          // both i32 → both widen to f64 → 2.5_f64
3.14_f32 / 2_i32       // i32 widens to f64; f32 widens to f64 → ~1.57_f64
5_i64 / 2_i64          // both i64 → both widen to f64 (pragmatic exception) → 2.5_f64
5.0_f64 / 2.0_f64      // both f64 → direct Div::div → 2.5_f64
```

The choice of which float type to widen to follows §4.5's mixed-kind rules:
the smaller integer widens to whichever float type matches the larger
operand, or to the default `f64` if both operands are integers without an
overriding context. Concretely:

- `i8`/`u8`/`i16`/`u16` operands widen to `f32` if the other operand is
  `f32`, or to `f64` otherwise (default and exact-representable).
- `i32`/`u32` operands widen to `f64` (exact-representable; `f32` would
  lose precision).
- `i64`/`u64` operands widen to `f64` (pragmatic exception; precision may
  be lost for values above 2⁵³).
- `i128`/`u128` operands: implicit widening is *not* permitted by §4.5;
  `/` on `i128`/`u128` operands requires an explicit cast to float first.
  The operator does not silently lose precision at the 128-bit boundary
  where the precision loss is dramatic.

If neither operand pins the float type and both are integers, the result is
a `Float`-placeholder value (§2.1) that resolves per §3.1.5's default
(`f64`) unless context demands otherwise.

This rule is the *only* place in the language where an operator performs
implicit kind-crossing promotion on uniformly-integer operands. Every other
operator with mixed-kind support requires at least one operand to already
be of the target kind; `/` is unique in always producing float regardless
of input kinds.

**Mixed widths and kinds.** When the operands are mixed-width and
mixed-kind (e.g., `i64 + f32`), the integer first widens to the smallest
float type capable of representing its full range (f64 for i64, by
§4.5.4's pragmatic exception), then the f32 widens to f64 per §4.5.3.
The result is f64. To force an alternate target type (e.g., truncating
the i64 to fit f32), use an explicit cast.

##### 4.4.1.2 Other arithmetic operators

`//` is the truncating integer division operator. It accepts `Integer`
operands and produces an `Integer` result. `5 // 2` produces `2`; `-5 // 2`
produces `-3` (toward negative infinity). `Float` operands are a type error.
For float-input integer-output behavior, the user explicitly converts via
`as` or `From`/`Into`.

`%` (remainder) accepts both kinds and produces a result of the same kind
as its operands. Mixed-kind operands promote per §4.5.

Unary `-` is defined on signed integers and floats only. Applying unary `-`
to an unsigned integer is a type error at compile time — silent wrap on
negation is rejected as a footgun source. To compute the additive inverse of
an unsigned value, the user explicitly converts to a signed type via `as`
or `From`/`Into` per §7.

Negative integer literals (e.g., `-5` in `let x: i8 = -5`) are not subject
to the unary-minus-on-unsigned rule. Per §2.4.5, `-N` is parsed as a single
signed literal token at type-check time, not as the unary operator applied
to a positive literal. The unary-minus rule applies only to runtime values
of unsigned type, never to literals at their declaration site.

#### 4.4.2 Bitwise operators

| Operator    | Operand Trait | Result                              |
|-------------|---------------|-------------------------------------|
| `&`         | `BitAnd`      | Integer (same type)                 |
| `\|`        | `BitOr`       | Integer (same type)                 |
| `^`         | `BitXor`      | Integer (same type)                 |
| `~` (unary) | `BitNot`      | Integer (same type)                 |
| `<<`        | `Shl`         | Integer (same type as left operand) |
| `>>`        | `Shr`         | Integer (same type as left operand) |

Bitwise operators are integer-only. Applying them to float values is a type
error. Bit-level operations on floats require an explicit reinterpret cast
through `as` to an integer type of the same width.

The `&` and `|` characters are reused at the type level (`&` for trait
intersection per §5, `|` as the leader of the placement attribute clause
per grammar §3.10 and for enum sum types per grammar §3.6). At the value
level — that is,
inside expressions — they are bitwise operators. The grammar's context-based
disambiguation determines which interpretation applies; user-visible
overloading is avoided through positional context.

At the value level, `|` is bitwise OR (dispatching through `BitOr`); the
operator-application token is `|>` (§13.17), a distinct token. Bitwise
`|` and `|>` share the same low precedence and left-associativity, so
expressions mixing bitwise OR with higher-precedence arithmetic parse
naturally; users mixing bitwise OR with operator application across the
same expression should add parentheses to make grouping explicit.

The right-shift operator `>>` is a single operator whose behavior depends
on the signedness of the left operand's type: signed types shift
arithmetically (sign-extending); unsigned types shift logically (zero-
extending). The compiler dispatches on the type via the `Shr` trait impl.
No separate `>>>` operator exists. `>>` has no other meaning at the value
level.

#### 4.4.3 Comparison operators

| Operator | Operand Trait | Result |
|----------|---------------|--------|
| `<`      | `Ord`         | bool   |
| `<=`     | `Ord`         | bool   |
| `>`      | `Ord`         | bool   |
| `>=`     | `Ord`         | bool   |

Comparison works on both integer and float kinds. Mixed-kind comparisons
promote per §4.5 before comparing. Float comparison follows IEEE 754
semantics including NaN behavior: `NaN < x` is `false`, `NaN > x` is `false`,
`NaN == NaN` is `false` (via the `is` operator below). This is a property of
IEEE 754, not a language design choice; user code working with potentially-
NaN floats must handle the NaN cases explicitly.

Comparison chaining (`a < b < c`) is not permitted (grammar §3.15 admits the
syntax but the type system rejects it: only the rightmost comparison is
typechecked as boolean-returning; intermediate comparisons in a chain would
produce a bool which then doesn't compare meaningfully with the next
operand).

#### 4.4.4 Equality operators

| Operator | Operand Trait | Result |
|----------|---------------|--------|
| `is`     | `Eq`          | bool   |
| `is not` | `Eq`          | bool   |

Equality uses the keyword forms `is` and `is not`, not symbolic `==`/`!=`
(grammar §3.15 and grammar §6 reserve symbolic equality against future
use). The keyword forms read more naturally in this language's
expression syntax and avoid the visual collision with `=` used for
binding-initialization.

Equality works on both integer and float kinds. Mixed-kind equality
promotes per §4.5 before comparing.

Float equality is permitted despite the precision hazards of IEEE 754
(`0.1 + 0.2 is not 0.3`). The alternative — removing `is`/`is not` from
floats and forcing epsilon comparison — is paternalistic and breaks
legitimate uses (NaN checks via `x is not x`, exact-zero comparisons,
comparisons against known-exact values). The hazard is documented; users
needing approximate comparison call stdlib `approx_eq(a, b, epsilon)` or
similar.

#### 4.4.5 Mixed-kind promotion (overview)

When an expression mixes integer and float operands, the integer operand
widens implicitly to the float type before the operation proceeds. Full
widening rules — both integer-to-integer and integer-to-float — are
specified in §4.5.

#### 4.4.6 Operator-to-inferred-constraint mapping

When the compiler infers constraints from a generic function body per
§2.2.2, each operator implies a specific trait constraint on its operands
(and on the result type where the operator produces a constrained result).
This table specifies the mapping:

| Operator                                    | Operand constraint                      | Result constraint                                    |
|---------------------------------------------|-----------------------------------------|------------------------------------------------------|
| `+`                                         | `Add`                                   | `Output` (associated type)                           |
| `-` (binary)                                | `Sub`                                   | `Output` (associated type)                           |
| `*`                                         | `Mul`                                   | `Output` (associated type)                           |
| `/`                                         | `Numeric`                               | `Self` on `Float` umbrella (per §4.4.1.1)            |
| `//`                                        | `IntDiv`                                | `Output` (associated type)                           |
| `%`                                         | `Rem`                                   | `Output` (associated type)                           |
| `-` (unary)                                 | `Neg`                                   | same type as operand                                 |
| `&`                                         | `BitAnd`                                | same type as operands                                |
| `\|`                                        | `BitOr`                                 | same type as operands                                |
| `^`                                         | `BitXor`                                | same type as operands                                |
| `~`                                         | `BitNot`                                | same type as operand                                 |
| `<<`                                        | `Shl` (left); `u32`-convertible (right)¹ | same type as left operand                           |
| `>>`                                        | `Shr` (left); `u32`-convertible (right)¹ | same type as left operand                           |
| `<`, `<=`, `>`, `>=`                        | `Ord`                                   | `bool`                                               |
| `is`, `is not`                              | `Eq`                                    | `bool`                                               |
| `+%`, `-%` (binary), `*%`, `//%`, `%%`      | corresponding `Wrapping...`             | same type as operands                                |
| unary `-%`                                  | `WrappingNeg`                           | same type as operand                                 |
| `+\|`, `-\|` (binary), `*\|`, `//\|`, `%\|` | corresponding `Saturating...`           | same type as operands                                |
| unary `-\|`                                 | `SaturatingNeg`                         | same type as operand                                 |
| `+?`, `-?` (binary), `*?`, `//?`, `%?`      | corresponding `Checked...`              | `Option[T]`                                          |
| `/?`                                        | `CheckedDiv` (on `Float`)               | `Option[Float]`; integer operands widen per §4.4.1.1 |
| unary `-?`                                  | `CheckedNeg`                            | `Option[T]`                                          |
| `as`                                        | (language-level)                        | the target type, traps on out-of-range               |
| `as%`                                       | `WrappingAs[T]` (operand)               | the target type T                                    |
| `as\|`                                      | `SaturatingAs[T]` (operand)             | the target type T                                    |
| `as?`                                       | `CheckedAs[T]` (operand)                | `Option[T]`                                          |

¹ The right operand may be any unsigned integer type narrower than or
equal to u32 (implicit widening per §4.5.1); other types require an
explicit cast.

Per §3.1.6's default-type-parameter resolution, each table entry that
names a bare trait (e.g., `Add`) refers to the trait instance with
default type parameters applied — `Add` is `Add[Self]`. For operands
of different types, the compiler may infer cross-type instances
(`Add[T2] for T1`) if such an instance is in scope; otherwise the
operand types must be unified per the implicit-widening rules of §4.5.

The compiler's inference algorithm per §2.2.1 walks each function body
collecting the union of these constraints across all operators used. The
resulting set is attached to the generic signature; call sites must satisfy
it. The umbrella traits from §4.9.2 may be substituted for sets of
fine-grained constraints when the substitution is unambiguous, for
readability in error messages and signatures.

For example, the body `a + (b - a) * c` infers `Add`, `Sub`, `Mul` on the
operand types (with the substitution rule that `a`, `b`, `c` are likely
related by inference — see §2.2.3). The compiler may report the inferred
bounds as `T: Numeric` rather than `T: Add + Sub + Mul + ...` when the
umbrella is unambiguous, but the underlying constraints are the
fine-grained traits per the operators used.

### 4.5 Implicit Widening

Implicit widening converts a narrower numeric value to a wider type
automatically, without an explicit cast, when the conversion is provably
lossless. All other conversions — narrowing, signed/unsigned crossing,
precision-losing — require explicit `as` (§4.7) or `From`/`Into` (§7).

The general principle: implicit widening fires only when the conversion
loses no information, with one pragmatic exception specified in §4.5.4.

#### 4.5.1 Integer-to-integer widening

| From                                            | To                                   | Implicit                 |
|-------------------------------------------------|--------------------------------------|--------------------------|
| `i8` → wider signed                             | `i16`, `i32`, `i64`, `i128`, `isize` | ✓                        |
| `u8` → wider unsigned                           | `u16`, `u32`, `u64`, `u128`, `usize` | ✓                        |
| `u8` → wider signed                             | `i16`, `i32`, `i64`, `i128`, `isize` | ✓ (always representable) |
| same-width signed/unsigned (e.g. `i32` ↔ `u32`) | the other                            | ✗ (explicit cast)        |
| signed → wider unsigned (e.g. `i8` → `u16`)     | wider unsigned                       | ✗ (negatives don't fit)  |
| any narrowing                                   | narrower type                        | ✗ (range may not fit)    |

The principle: same-signedness widening is implicit; unsigned-to-wider-signed
is implicit (always representable). Crossing signedness boundaries — even
when widening — requires an explicit cast, because the bit pattern's
interpretation changes (an unsigned value might not fit a signed range of
the same width; a negative signed value cannot represent in any unsigned
type).

#### 4.5.2 Integer-to-float widening

| From                                   | To        | Implicit                                       |
|----------------------------------------|-----------|------------------------------------------------|
| `i8`, `u8`, `i16`, `u16`               | `f32`     | ✓ (8/16-bit fits in f32's 24-bit mantissa)     |
| `i8`, `u8`, `i16`, `u16`, `i32`, `u32` | `f64`     | ✓ (up to 32-bit fits in f64's 53-bit mantissa) |
| `i32`, `u32`                           | `f32`     | ✗ (precision loss above 2²⁴)                   |
| `i64`, `u64`                           | `f64`     | ✓ (pragmatic exception — see §4.5.4)           |
| `i64`, `u64`                           | `f32`     | ✗ (significant precision loss)                 |
| `i128`, `u128`                         | any float | ✗ (significant precision loss)                 |

The rule: integer-to-float widening is implicit when the integer's full
range fits exactly in the float's mantissa. `f32` has a 24-bit mantissa, so
integer widths up to 16-bit are exactly representable; `f64` has a 53-bit
mantissa, so integer widths up to 32-bit are exactly representable.

#### 4.5.3 Float-to-float widening

| From  | To    | Implicit                                          |
|-------|-------|---------------------------------------------------|
| `f32` | `f64` | ✓ (exact-representable for all finite f32 values) |
| `f64` | `f32` | ✗ (precision and range loss)                      |

Float-to-float widening is implicit upward only. Narrowing from `f64` to
`f32` requires an explicit cast because both precision (mantissa width) and
range (exponent width) shrink.

#### 4.5.4 The i64/u64 → f64 pragmatic exception

`i64`/`u64` → `f64` is permitted as an implicit widening despite the formal
precision hazard for values above 2⁵³. The alternative — explicit casts on
every common `i64 + f64` expression — is more friction than the bounded
hazard justifies. The precision behavior is documented; users handling very
large integer magnitudes in float contexts are expected to be aware that
values exceeding 2⁵³ may lose low-order bits when converted to `f64`.

This is the only deviation from strict lossless widening. All other
precision-losing conversions require explicit casts.

#### 4.5.5 What requires explicit cast

Conversions not in §4.5.1–§4.5.3's implicit-widening tables require explicit
`as` (§4.7) or, for fallible conversions where the destination range might
not contain the source value, `TryFrom`/`TryInto` (§7) returning `Result`.

This includes: narrowing in either kind (wider-to-narrower integer,
wider-to-narrower float); signed/unsigned crossings of any width;
float-to-integer in any direction; precision-losing integer-to-float
(except the §4.5.4 exception); and any cross-type conversion involving
user-defined types via `From`/`Into`.

#### 4.5.6 Application: mixed-kind operators

The implicit-widening rules above are what makes mixed-kind operator
behavior work without explicit casts. For arithmetic operators (`+`, `-`,
`*`, `%`) with mixed-kind operands, the compiler applies the appropriate
widening from §4.5.1–§4.5.4 to bring operands to a common type, then
dispatches the operator's trait method on that type. For `/` specifically,
the operator's always-float result triggers integer-to-float widening even
for uniformly-integer operands per §4.4.1.1.

For comparison and equality operators (`<`, `<=`, `>`, `>=`, `is`,
`is not`), mixed-kind operands are widened the same way before comparison.

### 4.6 Overflow and Arithmetic Safety

Arithmetic operators have four variants per operation, expressing four
different policies for handling out-of-range results.

#### 4.6.1 Default trap-on-overflow

The default arithmetic operators (`+`, `-`, `*`, `/`, `//`, `%`, unary `-`)
trap on overflow at runtime, in all build modes. There is no debug-traps/
release-wraps distinction.

When an operation produces a result outside the destination type's range, the
runtime halts with a diagnostic identifying the operation, the operand
values, and the source location. Traps cannot be caught as values — see §8.

The performance cost of overflow checking on modern hardware is bounded
(a well-predicted branch per operation). The cost is accepted in exchange
for uniform semantics, safety in production, and the property that "this
code worked in testing" implies "this code is correct in production" for
overflow concerns.

#### 4.6.2 Wrapping operators

Wrapping operators perform modular two's-complement arithmetic, silently
wrapping on overflow:

| Operator   | Trait            | Behavior                                              |
|------------|------------------|-------------------------------------------------------|
| `+%`       | `WrappingAdd`    | `255_u8 +% 1 == 0_u8`                                 |
| `-%`       | `WrappingSub`    | `0_u8 -% 1 == 255_u8`                                 |
| `*%`       | `WrappingMul`    | `200_u8 *% 2 == 144_u8`                               |
| `//%`      | `WrappingIntDiv` | `(-128_i8) //% (-1_i8) == -128_i8` (no overflow trap) |
| `%%`       | `WrappingRem`    | rare; defined for completeness                        |
| unary `-%` | `WrappingNeg`    | `(-128_i8) -% == -128_i8` (no overflow trap)          |

Wrapping is the right choice for hash functions, cryptographic primitives,
counters where modular arithmetic is the intent, and bit-manipulation
patterns where wrap is mathematically meaningful.

Integer-division wrapping (`//%`) handles the one case where integer
division overflows: signed-minimum divided by `-1` (e.g., `i32::MIN // -1`,
which mathematically would be `2³¹` but doesn't fit in `i32`). The
wrapping form yields `i32::MIN` itself (the bit pattern wraps).

There is no `/%` for the `/` operator because `/` always produces `Float`
per §4.4.1.1, and float operations follow IEEE 754 (which doesn't
trap-overflow). No `//%` variant exists for division by zero — there is no
sensible modular answer to "divide by zero"; use `//?` (§4.6.4) for the
recoverable form, or accept that `//%` on a zero divisor traps.

#### 4.6.3 Saturating operators

Saturating operators clamp to the destination type's range bounds on
overflow:

| Operator    | Trait              | Behavior                            |
|-------------|--------------------|-------------------------------------|
| `+\|`       | `SaturatingAdd`    | `255_u8 +\| 1 == 255_u8`            |
| `-\|`       | `SaturatingSub`    | `0_u8 -\| 1 == 0_u8`                |
| `*\|`       | `SaturatingMul`    | `200_u8 *\| 2 == 255_u8`            |
| `//\|`      | `SaturatingIntDiv` | `(-128_i8) //\| (-1_i8) == 127_i8`  |
| `%\|`       | `SaturatingRem`    | rare; defined for completeness      |
| unary `-\|` | `SaturatingNeg`    | `(-128_i8) -\| == 127_i8`           |

Saturation is the right choice for DSP (audio sample clamping), image
processing (pixel value clamping), and any context where producing a
boundary value is preferable to either trapping or wrapping.

Integer-division saturation (`//|`) clamps the signed-min-divide-by-neg-one
overflow case to the type's maximum value, parallel to `//%`'s wrapping
behavior.

There is no `/|` for the `/` operator (same reasoning as `/%` above).
Saturating division by zero is not defined; use `//?` for the recoverable
form.

#### 4.6.4 Checked operators

Checked operators return `Option[T]` rather than producing a value-or-trap:

| Operator   | Trait           | Return          | Behavior                                                                    |
|------------|-----------------|-----------------|-----------------------------------------------------------------------------|
| `+?`       | `CheckedAdd`    | `Option[T]`     | `Some(result)` or `None`                                                    |
| `-?`       | `CheckedSub`    | `Option[T]`     | `Some(result)` or `None`                                                    |
| `*?`       | `CheckedMul`    | `Option[T]`     | `Some(result)` or `None`                                                    |
| `/?`       | `CheckedDiv`    | `Option[Float]` | `None` on NaN/Infinity result; integer operands widen to float per §4.4.1.1 |
| `//?`      | `CheckedIntDiv` | `Option[T]`     | `None` on overflow or div-by-zero                                           |
| `%?`       | `CheckedRem`    | `Option[T]`     | `None` on overflow or zero divisor                                          |
| unary `-?` | `CheckedNeg`    | `Option[T]`     | `None` on overflow                                                          |

The `/?` operator parallels `/` in widening behavior: integer operands
widen to float per §4.4.1.1, then dispatch to `CheckedDiv` on the float
type, returning `Option[Float]`. On float operands, the result is `None`
when IEEE 754 would produce `NaN` or `±Infinity` (e.g., divide by zero
producing `Infinity`, or `0.0/0.0` producing `NaN`); otherwise `Some(result)`.

The checked form is for cases where the caller wants to handle the
overflow or non-finite case explicitly without panicking. The `?` postfix
operator (§8) propagates the `None` upward in a function returning
`Option`-compatible types, making the recoverable-error chain ergonomic.

There are no `/%` or `/|` operators — wrapping and saturating
interpretations on float values would conflict with IEEE 754's
established semantics. Wrapping/saturating integer division uses `//%`
and `//|` per §4.6.2 and §4.6.3.

#### 4.6.5 Compile-time constant overflow

Compile-time constant overflow is always a compile error, regardless of
which operator variant is used. The compiler evaluates constant expressions
per §2.4 and rejects programs where a constant value provably doesn't fit
its declared or inferred type:

```
const x: u8 = 200_u8 + 100_u8                 // compile error: 300 doesn't fit u8
const x: u8 = 200_u8 +% 100_u8                // compile error: still doesn't fit
let arr: i32[some_large_compile_time_value]   // compile error if value doesn't fit isize
```

This applies to `+%`, `+|`, `+?` and other variants too: the compile-time
analysis happens before the runtime semantics of each variant matters.
Compile-time-known overflow is a programmer error to be fixed in code, not
a runtime condition to be handled.

#### 4.6.6 Float overflow

Float operations follow IEEE 754 semantics. Overflow produces signed
infinity (`f64::INFINITY` or `f64::NEG_INFINITY`); underflow may produce
subnormals or signed zero. NaN propagates through operations involving NaN
operands.

Float operators do not have wrapping or saturating variants — IEEE 754's
infinity-and-NaN semantics already define the overflow behavior, and
modular or clamping interpretations on float values would conflict with the
established standard. The checked variant `+?` etc. on floats is defined for
parity with integer checked operators and returns `None` if the operation
produces NaN or infinity (normative; see §4.7.3 for the analogous NaN
handling on saturating casts).

#### 4.6.7 Integer division by zero

Integer division by zero traps at runtime, per the default trap-on-overflow
philosophy. There is no sensible mathematical result for `n / 0` or `n // 0`
with integer types.

The checked variant `/?` (and `//?`) returns `None` for division by zero,
providing the recoverable form. There is no wrapping or saturating variant
for division by zero — no modular or clamping value is meaningful.

### 4.7 Explicit Casts

The `as` operator performs explicit numeric conversion. Like arithmetic
operators (§4.6), `as` has four variants expressing four different
out-of-range policies. The unsuffixed form is the default; suffixed forms
mirror the arithmetic operator suffixes.

#### 4.7.1 The four cast variants

| Operator | Trait             | Behavior on out-of-range                    |
|----------|-------------------|---------------------------------------------|
| `as`     | (language-level)  | trap at runtime                             |
| `as%`    | `WrappingAs[T]`   | modular two's-complement wrap               |
| `as\|`   | `SaturatingAs[T]` | clamp to destination type's range bounds    |
| `as?`    | `CheckedAs[T]`    | return `Option[T]` — `None` on out-of-range |

Examples:

```
let x: i32 = 300
let y: u8 = x as u8                  // ✗ traps at runtime — 300 doesn't fit u8
let y: u8 = x as% u8                 // ✓ wraps: 300 mod 256 == 44
let y: u8 = x as| u8                 // ✓ saturates to u8::MAX == 255
let y: Option[u8] = x as? u8         // ✓ None — out of range
let z: i32 = some_float as i32       // truncating float-to-int (may trap)
```

The trapping default matches §4.6.1's philosophy: in production code,
out-of-range cast is a bug to be surfaced, not silently transformed. Users
who want non-trapping behavior choose the appropriate variant explicitly.

#### 4.7.2 Lossless casts

For widening casts that are lossless per §4.5, `as` is the explicit-syntax
equivalent of implicit widening — the same result, no runtime cost beyond
the conversion itself. The variants (`as%`, `as|`, `as?`) on lossless
casts are equivalent to `as` (no out-of-range case can arise); they remain
syntactically valid for use in generic code where the cast's losslessness
isn't statically known.

#### 4.7.3 Float-to-integer casts

Float-to-integer casts via `as` truncate toward zero (matching most
language conventions). Out-of-range float values (NaN, infinity, values
larger than the integer's range) follow the variant's policy: `as` traps,
`as%` is implementation-defined (truncation modulo the destination range
is the typical choice), `as|` saturates to the destination's range bounds
(NaN treated as 0, parallel to §4.6.6's checked-arithmetic NaN handling),
`as?` returns `None`.

#### 4.7.4 Trait-based forms

Each operator variant has a corresponding trait method per §4.9.1:
`WrappingAs::wrapping_as`, `SaturatingAs::saturating_as`, `CheckedAs::checked_as`.
The methods are callable via uniform call syntax (§3.4) and produce the
same results as the operators:

```
let y: u8 = x.wrapping_as::[u8]()        // equivalent to `x as% u8`
let y: u8 = x.saturating_as::[u8]()       // equivalent to `x as| u8`
let y: Option[u8] = x.checked_as::[u8]()  // equivalent to `x as? u8`
```

The operators are the canonical user-facing form; the trait methods exist
for generic code that constrains on the trait, and as the underlying
dispatch targets the operators desugar to.

#### 4.7.5 `as` reserved for built-in numeric and newtype operations

`as` is reserved for two purposes:

- Built-in numeric conversion (§4.7.1–§4.7.4).
- Newtype extraction (§6.3.2).

These are dispatched by operand kind: a numeric primitive on the left side
uses the numeric-cast machinery; a newtype on the left side uses extraction.
User-defined conversions on non-newtype types go through the
`From`/`Into`/`TryFrom`/`TryInto` traits per §7. The `as` operator does
not extend to arbitrary user-defined conversions.

### 4.8 Special Numeric Operations

Operations beyond the core arithmetic operators (mathematical functions,
inspection methods, constants) are provided as trait methods on the relevant
numeric traits. Per §3.4 they are callable via method-call, conventional,
and trait-path syntax.

#### 4.8.1 General numeric operations

Available on all `Numeric` types (both integer and float):

| Operation | Trait | Signature                          |
|-----------|-------|------------------------------------|
| `abs`     | `Abs` | `fn abs(value: Self) -> Self`      |
| `min`     | `Min` | `fn min(a: Self, b: Self) -> Self` |
| `max`     | `Max` | `fn max(a: Self, b: Self) -> Self` |

Note on `abs`: applying `abs` to the minimum value of a signed integer type
(e.g., `i32::MIN.abs()`) traps on overflow per §4.6.1, because the
mathematical result (`2³¹`) doesn't fit in `i32`. The wrapping and
saturating variants are available as methods: `wrapping_abs`, `saturating_abs`.

`min` and `max` on floats are NaN-propagating by default. If either operand
is NaN, the result is NaN. This is consistent with every other float
operation in the language: any operation involving NaN produces NaN ("if
NaN in, NaN out"). Users with NaN-bearing data who want NaN to be ignored
in favor of the non-NaN operand opt in explicitly via `min_or` and `max_or`
(returning the non-NaN operand when exactly one is NaN, and NaN when both
are NaN).

This default aligns with IEEE 754-2019's recommended `minimum`/`maximum`
operations. The earlier IEEE 754-2008 `minNum`/`maxNum` operations (which
were NaN-suppressing) were deprecated in 2019 due to subtle issues with
negative zero and signaling NaN handling; the NaN-propagating form is now
the recommended primary behavior. The `min_or`/`max_or` variants implement
the older NaN-suppressing convention for data-processing use cases where
NaN represents missing data.

#### 4.8.2 Float-only operations

Available on `Float` types:

| Category      | Operations                                           |
|---------------|------------------------------------------------------|
| Square root   | `sqrt`                                               |
| Trigonometric | `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2` |
| Logarithmic   | `ln`, `log2`, `log10`, `log` (base, value)           |
| Exponential   | `exp`, `exp2`                                        |
| Rounding      | `floor`, `ceil`, `round`, `trunc`                    |
| Inspection    | `is_nan`, `is_infinite`, `is_finite`, `is_normal`    |

Each operation has its own trait (e.g., `Sqrt`, `Sin`, `Floor`). The
`Float` umbrella requires all of them per the umbrella pattern in §3.6.

Logarithm naming follows a deliberate convention to avoid the natural-vs-
base-10 ambiguity that plagues other languages: no bare `log(x)` exists.
Users write `ln(x)` for natural log, `log2(x)` for base-2, `log10(x)` for
base-10, and `log(base, x)` for arbitrary base. The two-argument `log`
takes the base as its first parameter.

Rounding operations (`floor`, `ceil`, `round`, `trunc`) are defined only on
floats. Integer ceiling division, floor division, and similar integer-domain
operations are standard-library concerns (e.g., a `div_ceil` method on
`Integer` if the stdlib provides it).

#### 4.8.3 Power operation

`pow` splits into two distinct traits based on operand kinds:

- `IntPow` (on `Integer`): integer base, integer exponent, integer result.
  Traps on overflow or on negative exponent (negative integer powers don't
  have integer results).
- `FloatPow` (on `Float`): float base, any-numeric exponent (integer
  promotes to float per §4.4.5), float result.

The typer picks the right trait based on the receiver's type. `2.pow(10)`
where `2` resolves to `i32` uses `IntPow`; `2.0.pow(0.5)` uses `FloatPow`.
The umbrella `Integer` includes `IntPow`; the umbrella `Float` includes
`FloatPow`.

A user calling `pow` with a negative integer exponent expecting a fractional
result must explicitly convert to float first:

```
let x = 2.pow(-1)              // ✗ compile error or trap: negative exponent on IntPow
let x = (2.0_f64).pow(-1)      // ✓ 0.5
let x = (2 as f64).pow(-1)     // ✓ 0.5
```

#### 4.8.4 Numeric constants

Constants live as associated values on the concrete numeric types, accessed
via path syntax:

```
f64::PI
f64::E
f64::TAU
f64::LN_2
f64::LN_10
f64::INFINITY
f64::NEG_INFINITY
f64::NAN
f32::PI
// ...
i32::MIN
i32::MAX
u8::MAX
i64::MIN
i64::MAX
// ...
```

Constants are associated with the concrete type rather than with traits
because their exact values depend on the type's representation (e.g.,
`f32::PI` and `f64::PI` differ in precision). Constants are `const`
declarations per §2.4.1.1, so they have no runtime storage and are inlined
at use sites.

### 4.9 The Numeric Trait Hierarchy

This section provides the concrete shape of the trait hierarchy referenced
throughout §3 and the preceding parts of §4. It instantiates the fine-
grained-plus-umbrella pattern from §3.6 for the numeric domain.

#### 4.9.1 Fine-grained operator traits

Each operator from §4.4 has its own trait, with the method name matching
the conventional operator name:

```
trait Add[Rhs = Self]:    type Output = Self; fn add(a: Self, b: Rhs) -> Output
trait Sub[Rhs = Self]:    type Output = Self; fn sub(a: Self, b: Rhs) -> Output
trait Mul[Rhs = Self]:    type Output = Self; fn mul(a: Self, b: Rhs) -> Output
trait Div:                fn div(a: Self, b: Self) -> Self      -- on Float umbrella only; see §4.4.1.1 for widening
trait IntDiv[Rhs = Self]: type Output = Self; fn intdiv(a: Self, b: Rhs) -> Output
trait Rem[Rhs = Self]:    type Output = Self; fn rem(a: Self, b: Rhs) -> Output
trait Neg:                fn neg(value: Self) -> Self

trait BitAnd: fn bitand(a: Self, b: Self) -> Self
trait BitOr:  fn bitor(a: Self, b: Self) -> Self
trait BitXor: fn bitxor(a: Self, b: Self) -> Self
trait BitNot: fn bitnot(value: Self) -> Self
trait Shl:    fn shl(value: Self, n: u32) -> Self
trait Shr:    fn shr(value: Self, n: u32) -> Self

trait WrappingAdd[Rhs = Self]:    type Output = Self; fn wrapping_add(a: Self, b: Rhs) -> Output
trait WrappingSub[Rhs = Self]:    type Output = Self; fn wrapping_sub(a: Self, b: Rhs) -> Output
trait WrappingMul[Rhs = Self]:    type Output = Self; fn wrapping_mul(a: Self, b: Rhs) -> Output
trait WrappingIntDiv[Rhs = Self]: type Output = Self; fn wrapping_intdiv(a: Self, b: Rhs) -> Output
trait WrappingRem[Rhs = Self]:    type Output = Self; fn wrapping_rem(a: Self, b: Rhs) -> Output
trait WrappingNeg:                fn wrapping_neg(value: Self) -> Self

trait SaturatingAdd[Rhs = Self]:    type Output = Self; fn saturating_add(a: Self, b: Rhs) -> Output
trait SaturatingSub[Rhs = Self]:    type Output = Self; fn saturating_sub(a: Self, b: Rhs) -> Output
trait SaturatingMul[Rhs = Self]:    type Output = Self; fn saturating_mul(a: Self, b: Rhs) -> Output
trait SaturatingIntDiv[Rhs = Self]: type Output = Self; fn saturating_intdiv(a: Self, b: Rhs) -> Output
trait SaturatingRem[Rhs = Self]:    type Output = Self; fn saturating_rem(a: Self, b: Rhs) -> Output
trait SaturatingNeg:                fn saturating_neg(value: Self) -> Self

trait CheckedAdd[Rhs = Self]:    type Output = Self; fn checked_add(a: Self, b: Rhs) -> Option[Output]
trait CheckedSub[Rhs = Self]:    type Output = Self; fn checked_sub(a: Self, b: Rhs) -> Option[Output]
trait CheckedMul[Rhs = Self]:    type Output = Self; fn checked_mul(a: Self, b: Rhs) -> Option[Output]
trait CheckedDiv[Rhs = Self]:    type Output = Self; fn checked_div(a: Self, b: Rhs) -> Option[Output]
trait CheckedIntDiv[Rhs = Self]: type Output = Self; fn checked_intdiv(a: Self, b: Rhs) -> Option[Output]
trait CheckedRem[Rhs = Self]:    type Output = Self; fn checked_rem(a: Self, b: Rhs) -> Option[Output]
trait CheckedNeg:                fn checked_neg(value: Self) -> Option[Self]

trait WrappingAs[T]:   fn wrapping_as(value: Self) -> T
trait SaturatingAs[T]: fn saturating_as(value: Self) -> T
trait CheckedAs[T]:    fn checked_as(value: Self) -> Option[T]

trait Zero: fn zero() -> Self
trait One:  fn one() -> Self

trait Abs:  fn abs(value: Self) -> Self
trait Min:  fn min(a: Self, b: Self) -> Self
trait Max:  fn max(a: Self, b: Self) -> Self

trait Sqrt: fn sqrt(value: Self) -> Self
trait Sin:  fn sin(value: Self) -> Self
trait Cos:  fn cos(value: Self) -> Self
// ... and so on for the float-only operations from §4.8.2

trait IntPow:   fn pow(base: Self, exp: Self) -> Self
trait FloatPow: fn pow(base: Self, exp: Self) -> Self

trait Eq: fn eq(a: Self, b: Self) -> bool

trait Ord: requires Lt, Le, Gt, Ge
trait Lt: fn lt(a: Self, b: Self) -> bool
trait Le: requires Lt, Eq
          fn le(a: Self, b: Self) -> bool:
            lt(a, b) or eq(a, b)
trait Gt: requires Lt, Eq
          fn gt(a: Self, b: Self) -> bool:
            not (lt(a, b) or eq(a, b))
trait Ge: requires Lt
          fn ge(a: Self, b: Self) -> bool:
            not lt(a, b)
```

This is the canonical fine-grained set. Stdlib may add additional fine-
grained traits for specialized operations; the principle (one trait per
capability) is what's normative, not the exact list above.

Note: `Div` is non-generic (Float-umbrella only); `CheckedDiv` is generic
for consistency with other checked-arithmetic traits, but instances are
auto-derived only for Float types where `Div` is implemented.

`Ord` and `Eq` are standalone — not part of any numeric umbrella per §3.6.1.
Non-numeric types (strings, enums, records) may also be ordered or compared,
so these traits live outside the numeric hierarchy.

`Ord` is an umbrella per §3.3.5: it requires the four ordering traits and
declares no methods of its own. A type satisfies `Ord` automatically when it
satisfies `Lt`, `Le`, `Gt`, `Ge`. In practice, implementers fulfill `Lt` and
`Eq` only — the default bodies on `Le`, `Gt`, `Ge` derive their behavior from
`Lt::lt` and `Eq::eq` per §3.1.3. Auto-derivation via `@derive(Ord)` per
§3.8 generates the full set of fulfill blocks structurally; manual
implementation requires only `fulfill Lt for X` and `fulfill Eq for X`.

The `is not` operator does not have its own trait method. `a is not b`
desugars at parse time to `not (a is b)` and dispatches through `Eq::eq` per
the operator semantics in §4.4.4. This preserves the one-method-per-trait
pattern: `Eq` declares one method (`eq`); the two operators `is` and `is not`
both flow through it.

#### 4.9.2 Umbrella traits

Umbrella traits combine fine-grained traits via `requires` clauses (§3.1.4),
introducing no new methods of their own. They are pure-requirement traits
per §3.3.5: automatically satisfied when all required traits are satisfied.

```
@default(i32)
trait Numeric:
  requires Add, Sub, Mul, Zero, One,
           WrappingAdd, WrappingSub, WrappingMul,
           SaturatingAdd, SaturatingSub, SaturatingMul,
           CheckedAdd, CheckedSub, CheckedMul,
           Abs, Min, Max

@default(i32)
trait Integer:
  requires Numeric, Rem, IntDiv, BitAnd, BitOr, BitXor, BitNot, Shl, Shr,
           WrappingRem, WrappingIntDiv,
           SaturatingRem, SaturatingIntDiv,
           CheckedIntDiv, CheckedRem,
           IntPow

@default(f64)
trait Float:
  requires Numeric, Neg, Div,
           CheckedDiv, CheckedNeg,
           Sqrt, Sin, Cos, Tan, Asin, Acos, Atan, Atan2,
           Ln, Log2, Log10, Exp, Exp2,
           Floor, Ceil, Round, Trunc,
           FloatPow

@default(i32)
trait Signed:
  requires Integer, Neg, WrappingNeg, SaturatingNeg, CheckedNeg

@default(u32)
trait Unsigned:
  requires Integer
  // Unsigned does NOT require Neg; types satisfying Unsigned do not
  // implement Neg, so unary `-` on them is a type error per §4.4.1
```

`Neg` is deliberately not part of `Numeric`. Unsigned integer types cannot
implement `Neg` (§4.4.1: unary `-` on unsigned is a type error), so placing
`Neg` in `Numeric` would prevent unsigned types from satisfying the
`Numeric` umbrella. The clean resolution: `Numeric` collects only the
operations meaningful for both signed and unsigned numbers; `Neg` (and its
wrapping/saturating/checked variants) appear on `Signed` and `Float`
separately. The signed/unsigned distinction is then exactly the presence
or absence of `Neg` in the type's effective method set: types satisfying
`Signed` implement `Neg`; types satisfying `Unsigned` do not; floats
implement `Neg` via the `Float` umbrella.

`Div` is on `Float` only (not on `Integer` or `Numeric`), reflecting Topic
5's rule that `/` always produces `Float`. Integer operands to `/` are
implicitly widened to float per §4.4.1.1 before `Div::div` is dispatched.

#### 4.9.3 Default mappings

Defaults declared on the umbrella traits per §3.1.5 are confirmed against
the final type set:

| Trait      | Default Type | Rationale                                     |
|------------|--------------|-----------------------------------------------|
| `Numeric`  | `i32`        | Workhorse general-purpose integer             |
| `Integer`  | `i32`        | Same                                          |
| `Float`    | `f64`        | Higher precision preferred when unconstrained |
| `Signed`   | `i32`        | Workhorse signed integer                      |
| `Unsigned` | `u32`        | Symmetric counterpart to `i32`                |

The `i32` and `f64` defaults match modern language convention (Rust, Swift,
Kotlin, C#) and reflect the types where the cost/precision tradeoffs are
most balanced for general code.

#### 4.9.4 Auto-implementations for built-in numeric types

The fourteen built-in numeric types auto-implement the appropriate
fine-grained traits per §3.3 (auto-impls of built-in numeric traits for
built-in numeric types). Umbrella satisfaction follows transitively per
§3.3.5.

Specifically:

- **All integer types** auto-implement: `Add`, `Sub`, `Mul`, `Rem`,
  `IntDiv`, `BitAnd`, `BitOr`, `BitXor`, `BitNot`, `Shl`, `Shr`; the
  wrapping variants `WrappingAdd`, `WrappingSub`, `WrappingMul`,
  `WrappingIntDiv`, `WrappingRem`; the saturating variants
  `SaturatingAdd`, `SaturatingSub`, `SaturatingMul`, `SaturatingIntDiv`,
  `SaturatingRem`; the checked variants `CheckedAdd`, `CheckedSub`,
  `CheckedMul`, `CheckedIntDiv`, `CheckedRem` (note: not `CheckedDiv`,
  which is float-only since `/` widens integers to float per §4.4.1.1);
  the cast traits `WrappingAs`, `SaturatingAs`, `CheckedAs`; `Zero`,
  `One`, `Abs`, `Min`, `Max`, `Ord`, `Eq`, `IntPow`; and (for signed
  integer types) `Neg`, `WrappingNeg`, `SaturatingNeg`, `CheckedNeg`.
  They satisfy `Integer`, `Numeric`, and `Signed` or `Unsigned`
  accordingly.
- **Float types** auto-implement: `Add`, `Sub`, `Mul`, `Div`, `Rem`,
  `Neg`; the checked variants `CheckedAdd`, `CheckedSub`, `CheckedMul`,
  `CheckedDiv`, `CheckedNeg` (returning `None` on NaN or Infinity
  results per §4.6.6); the cast trait `WrappingAs[T]` for integer
  destination types `T` (per §4.7.3 — float-to-integer with
  implementation-defined modular truncation) and for wider-float
  destinations (trivially equivalent to `as` per §4.7.2 since the
  conversion is lossless); the cast traits `SaturatingAs[T]` and
  `CheckedAs[T]` for integer destinations (clamping NaN to 0, etc.,
  per §4.7.3), narrower-float destinations (saturation clamps to the
  destination's range bounds; checked returns `None` on overflow), and
  wider-float destinations (trivially equivalent to `as` per §4.7.2).
  `WrappingAs[T]` is *not* implemented for narrower-float destinations
  because modular wrap has no sensible meaning when the destination has
  reduced range and precision. Plus: float-only operations (`Sqrt`,
  trig, log, exp, rounding); inspection methods; `Zero`, `One`, `Abs`,
  `Min`, `Max`, `Ord`, `Eq`, `FloatPow`. Floats do not implement
  `WrappingAdd` / `SaturatingAdd` etc. — IEEE 754's infinity-and-NaN
  semantics already define overflow behavior, and modular or clamping
  interpretations would conflict (§4.6.6). They satisfy `Float`,
  `Numeric`, and `Signed` (floats are signed by convention — they
  support `Neg`).

Built-in numeric types implement the same-type instantiations only — e.g.,
`fulfill Add[i32] for i32`, not `Add[i64] for i32`. Cross-type arithmetic
between built-in numeric types requires explicit conversion (§4.5, §4.7);
user-defined types may implement cross-type instantiations such as
`Add[i32] for Distance` per §6.3.3.

User-defined numeric-like types (`Decimal` from stdlib, custom fixed-point
types, etc.) implement whichever fine-grained traits are appropriate;
umbrella satisfaction follows.

---

## 5. Type Intersection and `dyn`

The `&` operator expresses type intersection — "satisfies all of these
simultaneously" — and appears in three distinct contexts with related but
position-dependent semantics. The unifying intuition is uniform; the
concrete meaning varies by what the operands are and where the expression
sits.

The three contexts:

| Context                                | Operands             | Example                    |
|----------------------------------------|----------------------|----------------------------|
| Generic bound                          | Traits               | `fn pick[T: A & B](...)`   |
| Value-position trait object            | Traits, behind `dyn` | `let x: dyn (A & B) = ...` |
| Record intersection at type definition | Records              | `type X = A & B`           |

### 5.1 Trait Conjunction in Generic Bounds

In a generic parameter list or where-clause, `T: A & B` constrains `T` to
be a type for which both `fulfill A for T` and `fulfill B for T` exist:

```
fn pick[T: Drivable & Insurable](item: T) -> T:
  ...

fn process[T](item: T) where T: Drivable & Insurable:
  ...
```

The `&` here is *constraint conjunction*, not a type expression. The
compiler resolves it statically at every use site; instantiations are
monomorphized per §2.3 with no runtime dispatch cost. A type either
satisfies all conjoined constraints or it doesn't; the constraint set is
checked at the call site for each concrete instantiation.

Conjunction is commutative and associative: `A & B`, `B & A`, and
`(A & B) & C` are equivalent constraint sets.

### 5.2 Trait Objects at Value Position (`dyn`)

A trait may appear at value position — as the type of a variable, parameter,
field, or return value — only when wrapped in `dyn`. The resulting *trait
object* dispatches method calls dynamically through a vtable.

#### 5.2.1 Single-trait and multi-trait forms

Single-trait `dyn`:

```
let x: dyn Drivable = some_value
fn render(item: dyn Renderable) -> string: ...
```

Multi-trait `dyn` (intersection at value position):

```
let x: dyn (Drivable & Insurable) = some_value
fn process(item: dyn (Drivable & Insurable)) -> dyn Renderable: ...
```

When `dyn` precedes an intersection of traits, the intersection MUST be
parenthesized. Without parens, `dyn Drivable & Insurable` parses as
`(dyn Drivable) & Insurable` — `dyn Drivable` becomes a trait object,
which is then intersected with the bare trait `Insurable`, which is
ill-formed (trait objects are not in the `{trait & trait}` intersection
domain per §5.5). The parens force the intended grouping: `dyn` applied
to the trait-intersection expression as a whole.

#### 5.2.2 `dyn` is mandatory for trait-object value positions

`dyn` is *required* at every trait-object value position. The bare form
`let x: Drivable` (no `dyn`) is a parse error when `Drivable` is a trait
rather than a concrete type. Similarly, `let x: Drivable & Insurable`
(no `dyn`) is a parse error when both operands are traits.

The requirement makes dynamic-dispatch costs visible at the declaration
site rather than hidden behind syntax that looks like a plain type
annotation. Users who want static dispatch use generics with trait bounds
per §5.1; users who want dynamic dispatch use `dyn` per §5.2 and pay the
indirection cost knowingly.

#### 5.2.3 Dispatch cost

Trait objects dispatch through a vtable. The runtime cost is an indirect
call per method invocation, plus the storage cost of the vtable pointer
adjacent to the value's data. The costs are bounded and predictable;
they are simply not zero, which is the property `dyn` makes visible.

#### 5.2.4 Object safety

Not every trait can be used in a `dyn` position. Traits with methods whose
signatures depend on `Self` in non-receiver positions, traits with
associated types not bound at the use site, or traits with generic methods
cannot be made into trait objects under the standard vtable mechanism.
Object-safety rules are specified in detail in [Object Safety: deferred
to a future spec revision]. A trait that is not object-safe used in a
`dyn` position produces a compile error at the use site identifying the
offending trait and the reason.

#### 5.2.5 Coercion to `dyn`

A value of a concrete type `T` that fulfills traits `A` and `B` can be
assigned to a `dyn (A & B)` binding via an explicit coercion. The exact
syntax is specified in [Coercion: deferred to a future spec revision];
the principle is that moving from a static type to a trait object is a
deliberate operation at the assignment or argument-passing site, not an
implicit conversion.

### 5.3 Record Intersection at Type Definition

A `type` declaration whose right-hand side is a record-record intersection
produces a new nominal record type combining the fields of both operands:

```
type Car:
  brand: string
  speed: f64
  wheels: i32

type Insured:
  policy_number: string
  premium: f64

type InsuredCar = Car & Insured

// Equivalent declaration:
type InsuredCar:
  brand: string
  speed: f64
  wheels: i32
  policy_number: string
  premium: f64
```

The resulting `InsuredCar` is *nominally distinct* from both `Car` and
`Insured`. Values of `Car` are not implicitly assignable to `InsuredCar`
(a `Car` lacks the insurance fields); values of `InsuredCar` are not
implicitly assignable to `Car` (no implicit projection of fields).
Conversion requires explicit construction or a `From` impl per §7.

The intersection is a *definitional combinator* producing a new named type,
not a subtyping relationship. The language has no nominal subtyping; record
intersection composes structure into a new identity, full stop.

#### 5.3.1 Field merging rules

When both operand records declare a field with the same name:

- **Identical types and identical visibility** — the merged record has a
  single field of that name, type, and visibility. No duplication.
- **Different types** — the intersection is a compile error identifying the
  conflicting field name and the two incompatible types. The user resolves
  by writing the record explicitly with the chosen field type, or by
  adjusting the source records.
- **Same type, different visibility** — the intersection is a compile error
  identifying the conflicting field name and the two incompatible visibility
  specifiers. Visibility is part of a field's contract per §10; the two
  operand records disagree about how the field should be exposed, and the
  merged record cannot resolve the disagreement without arbitrarily picking
  one. The user resolves by writing the record explicitly with the chosen
  visibility or by aligning the source records.

#### 5.3.2 Trait inheritance via `@derive`

Trait inheritance from the operand records is opt-in via `@derive` per
§3.8. Each trait to be inherited is explicitly listed in the annotation,
and the compiler generates the `fulfill` block by delegating to the
operand types' implementations:

```
@derive(Display, Hash)
type InsuredCar = Car & Insured
```

When both operand records have `fulfill` blocks for the same trait that
would equally apply, derivation is ambiguous; the compiler reports an error
and the user must write the implementation manually.

The mechanism mirrors `@derive` for newtypes (§6.3.3): explicit
opt-in trait inheritance, no automatic carry-over of traits the user didn't
ask for.

### 5.4 Interaction with `alias type`

The `alias type` mechanism (§4.2; contrasted with newtypes in §6.3)
produces transparent
substitution — the alias name and its right-hand side refer to the same
thing, no new identity. Interaction with `&` depends on what the right-hand
side evaluates to:

- **`alias type X = A & B` where A, B are traits** — valid. The alias names
  a *constraint* usable in bound positions. `fn process[T: X](item: T)` is
  equivalent to `fn process[T: A & B](item: T)`. Useful for naming common
  bounds for reuse.
- **`alias type X = dyn (A & B)` where A, B are traits** — valid. The alias
  names a *dynamic-dispatch trait object type*. `let value: X = ...` is
  equivalent to `let value: dyn (A & B) = ...`.
- **`alias type X = A & B` where A, B are records** — rejected at compile
  time. Record intersection creates a new nominal type with combined
  fields; that creation requires `type`, not `alias type`. Without new
  identity, the intersection has no meaning in the nominal type system.
  The compile error directs the user to write `type X = A & B` instead.

The asymmetry between trait intersection (aliasable) and record
intersection (not aliasable) reflects a deeper asymmetry: trait
intersection produces a constraint (or a `dyn` type with explicit
identity); record intersection produces fields-combined-into-a-new-type
that has meaning only as a nominal type with identity. Aliases work where
the right-hand side already has identity; they don't work where the
right-hand side requires a type declaration to acquire identity.

### 5.5 Cross-Kind Intersection

Intersection is well-defined only within `{trait & trait}` (trait
intersection) and `{record & record}` (record intersection). Cross-kind
combinations and same-kind combinations outside those two sets are
rejected at compile time:

- `Trait & Record` — rejected. A trait expresses a behavior contract; a
  record expresses structure. Their intersection has no coherent meaning.
- `Record & Enum` — rejected. Records and enums are distinct compound
  kinds; combining them produces no type the language can represent.
- `Trait & Enum` — rejected. Same reasoning.
- `Enum & Enum` — rejected. Enums are tagged unions; intersection of two
  tagged unions has no meaningful semantics (the union of their variants
  would be `|`-shaped, not `&`-shaped, and is not provided by the language).
- Intersections involving tuples, function types, or primitive types —
  rejected. These kinds are not subject to intersection.

The compiler reports the cross-kind intersection error at the `&`
expression with the operand kinds named.

### 5.6 Variance and Intersection

The language has no variance markers and no subtyping between generic
instantiations (§2.3). `Container[Cat]` and `Container[Animal]` are
unrelated types regardless of any relationship between `Cat` and `Animal`.

Intersection of two distinct generic instantiations (e.g.,
`Container[Cat] & Container[Animal]`) follows the rules for the resulting
kinds. As record intersection, the operands' fields would typically
conflict (different generic instantiations differ in their field types per
§2.3.1's strict-structural keying), so most such expressions are compile
errors via §5.3.1's same-name-different-type rule. As trait conjunction,
the conjunction is well-formed but produces a constraint that may have no
satisfying type — generic constraints don't fail at the constraint
declaration; they fail at the call site where no concrete type satisfies
both.

---

## 6. Records, Enums, and Newtypes

This section specifies the language's three user-defined nominal compound
types: records (product types), enums (sum types), and newtypes (wrapper
types). All three are nominal — identity is by name, not structure — and
all three participate uniformly in the trait system per §3.

### 6.1 Records

A record is a nominal product type with a fixed set of named fields. Records
carry data only; they have no methods of their own. Behavior associated with
a record's type is expressed via free functions and trait implementations
that the record satisfies, per §3.

#### 6.1.1 Declaration

A record is declared with the `type` keyword followed by the type name and a
body of field declarations (grammar §3.5):

```
type Person:
  first_name: string
  last_name: string
  age: i32

type Vec3:
  x: f64
  y: f64
  z: f64

type Point[T]:
  x: T
  y: T
```

Each field declares a name, a type, and optionally a default value. The
field type may be any type expression — primitive, record, enum, generic
parameter, trait object, or compound. A record may declare generic
parameters in the standard `[T, U, ...]` form; each generic parameter is
in scope within the field declarations.

A record body may include a `satisfies` clause listing the traits the type
promises to implement (§3.2):

```
type Person:
  satisfies Display, Hash, Eq
  first_name: string
  last_name: string
  age: i32
```

The `satisfies` clause may appear once per record, conventionally at the
top of the body. Per §3.2, every trait listed must have a matching `fulfill`
block reachable through the module graph; pure-requirement umbrella traits
per §3.3.5 are satisfied automatically when their requirements are.

Records do not declare methods. Functions operating on record instances are
free functions defined elsewhere (grammar §3.13) or trait-method
implementations in `fulfill` blocks (§3.3). The uniform function call
syntax (§3.4) makes these callable as `x.f()` or `f(x)`
indifferently.

#### 6.1.2 Field defaults

A field may declare a default value:

```
type Window:
  title: string
  width: i32 = 800
  height: i32 = 600
  resizable: bool = true
```

A default value is any expression valid at the record's declaration scope.
Per §2.4.1, defaults that are compile-time-known (the typical case) are
evaluated and inlined at construction sites where the field is omitted;
defaults involving runtime values are evaluated at each construction.

Defaults compose with construction (§6.1.3): a field with a default may be
omitted at the construction site, in which case the default applies.

#### 6.1.3 Construction

A record value is constructed by calling the type name with named arguments:

```
let alice = Person(
  first_name: "Alice",
  last_name: "Smith",
  age: 30,
)

let w = Window(title: "Main")  // width, height, resizable use their defaults
```

Field arguments are named, not positional. The order of named arguments
does not matter. Every field without a default must be supplied; supplying
the same field twice is a compile error; supplying an unknown field name is
a compile error.

Positional construction is not supported. Records are nominal product types
with named fields; positional construction would obscure which value goes
into which field, especially for records with many fields or fields of the
same type. The explicit-name requirement is verbose at small record sizes
but scales cleanly.

Generic records require concrete type arguments at construction. The
arguments may be inferred from the field types or supplied explicitly:

```
let p: Point[f64] = Point(x: 1.0, y: 2.0)         // T inferred from arguments
let q = Point::[i32](x: 1, y: 2)                  // T explicit via turbofish
```

#### 6.1.4 Field access

A field is accessed by dot notation: `record.field_name`. The dot operator
is the field-access operator, distinct from the method-call operator (which
also uses `.` but is followed by a function name and call syntax). The
compiler disambiguates by the syntax following the dot.

Field access is read-only. A record's fields cannot be reassigned after
construction; the binding's immutability (§1.3) applies transitively to the
fields. To produce a modified record, the user constructs a new record
value, typically via the record-update expression `with` (§6.1.5).

#### 6.1.5 Record update with `with`

The `with` expression produces a new record value derived from an existing
one with selected fields overridden or merged from other records:

**Single-line form (comma-separated):**

```
let updated = base with name: "new"
let updated = base with name: "new", age: 30
let updated = base with other
let updated = base with other1, other2
let updated = base with other1, other2, name: "new", age: 30
```

**Multi-line form (colon-introduced body):**

```
let updated = base with:
  name: "new"
  age: 30

let updated = base with other1, other2:
  name: "new"
  age: 30
```

These are the only two surface forms. Mixing single-line and multi-line in
one expression is a parse error.

The expression's components, evaluated left to right:

- The *base* (`base`) — a record value whose type defines the result type.
- Zero or more *merge sources* (other record values like `other1`,
  `other2`) — each must be of the same type as the base; fields are
  copied into the result.
- Zero or more *field overrides* (`name: "new"`) — each override sets one
  field of the result.

The result is a new record of the base's type. Merge sources and field
overrides are applied left-to-right; later assignments win on conflict.
For `base with other1, other2, name: "new"`:

1. Start with `base`'s field values.
2. Override with `other1`'s field values.
3. Override with `other2`'s field values.
4. Override `name` with `"new"`.

A field unset in any source/override keeps the base's value. The result is
always the same record type as the base.

##### Same-type constraint

All merge sources must have the *exact same type* as the base. Cross-type
merge is a compile error. The `with` expression does not create new types
at runtime; the language's type system is static.

```
let car_2: Car = car_1 with car_3        // ✓ both Car
let bad = car with insured_record         // ✗ Car and Insured are different types
```

For combining different types' fields into a new type, the user constructs
a record-intersection type per §5.3 and constructs values of it
explicitly.

When `with` appears in a reactive declaration context, additional
per-field rules apply: bare reactive names alias, reactive expressions
become implicit derived cells, and literals/non-reactive values become
static fields. See §13.2.9.8 for the reactive-context extension.

##### Field-override constraints

Every override field name must exist in the base's type. Overriding a
non-existent field is a compile error. Override values must be type-
compatible with the field's declared type (subject to the same widening
and conversion rules as direct construction per §6.1.3).

#### 6.1.6 Field visibility

Each field carries an independent visibility specifier per §10:

```
type Account:
  public id: i64                  // readable anywhere the type is visible
  email: string                   // shared (default) — readable within package
  private password_hash: string   // readable only within this module
```

Field visibility is independent from the enclosing type's visibility and
from the constructor's visibility (§6.1.7). A field's visibility never
exceeds the enclosing type's visibility — declaring a `public` field on a
`private` type is a compile error, because no caller outside the type's
visibility scope could observe the field.

A field accessed from outside its visibility scope produces a compile
error. The error is at the access site, not at the record's declaration.

#### 6.1.7 Constructor visibility

The constructor's visibility is independently controllable from the type's
visibility per §10's `public(constructor_vis)` mechanism:

```
public type Email:                            // both public
  wraps string

public(private) type Email:                   // type public, constructor private
  wraps string                                // (smart-constructor pattern)

shared(private) type SecretConfig:            // type shared, constructor private
  api_key: string
  endpoint: string
```

When the constructor is private, the type's name is visible but the
construction syntax `TypeName(...)` is unreachable from outside the
constructor's scope. The pattern enables types whose values can only be
created through controlled paths — typically via a `From` impl or a
factory function (§7).

Constructor visibility never exceeds type visibility; an inner specifier
more permissive than the outer is a compile error.

#### 6.1.8 Trait auto-derivation

Per §3.8, the `@derive` annotation generates structural trait
implementations for a fixed set of traits:

```
@derive(Eq, Ord, Hash, Clone, Display, Debug)
type Person:
  first_name: string
  last_name: string
  age: i32
```

Derivation operates field-by-field per §3.8.2: each field's type must
itself satisfy the trait being derived. Derivation failure (a field whose
type doesn't satisfy the trait) is a compile error identifying the
offending field.

Some derivable traits have dependencies on others. Deriving `Ord` requires
`Eq` to also be available on the same type — either by being derived in
the same annotation or by being satisfied through a manual `fulfill`
block. This dependency reflects the implementation: `Ord`'s default
bodies for `Le`, `Gt`, `Ge` (per §4.9.1) call `Eq::eq`. Deriving `Ord`
without `Eq` is a compile error identifying the missing dependency.

#### 6.1.9 Records and trait dispatch

A record's behavior — equality, hashing, display, comparison, conversion,
domain-specific operations — is delivered through trait implementations,
not through methods declared on the record. The implementations live in
`fulfill` blocks per §3.3, and dispatch through uniform function call
syntax per §3.4. The record's body is restricted to data.

This separation is structural, not stylistic: a record body cannot contain
`fn` declarations. Functions that operate on a record live as free
functions or as `fulfill`-block methods, never inside the record's body.

### 6.2 Enums

An enum is a nominal sum type — a tagged union of variants. Each variant
has a name and an optional payload of types. A value of the enum is exactly
one of the declared variants at any time; pattern matching (§6.2.4) is the
canonical way to inspect which.

#### 6.2.1 Declaration

An enum is declared with the `enum` keyword (grammar §3.6):

```
enum Direction:
  North
  South
  East
  West

enum Shape:
  Circle(f64)                              // positional payload
  Rectangle(width: f64, height: f64)       // named payload
  Triangle(f64, f64, f64)                  // positional payload

enum Result[T, E]:
  Ok(T)
  Err(E)

enum Option[T]:
  Some(T)
  None
```

Each variant declares a name (PascalCase, like a type name) and zero or
more payload fields. Payload fields may be declared in two forms:

- **Positional payload** — the type alone, with no name:
  `Circle(f64)`, `Ok(T)`, `Triangle(f64, f64, f64)`.
- **Named payload** — name and type, parallel to record fields:
  `Rectangle(width: f64, height: f64)`.

A variant with no payload is a *unit variant* (`North`, `None`).

Within a single variant's payload declaration, the form is uniform: all
positional or all named. Mixing within one variant declaration is a
compile error:

```
enum Bad:
  Mixed(width: f64, f64)         // ✗ compile error — mixed declaration
```

Different variants of the same enum may use different forms independently,
as `Shape` above shows.

##### 6.2.1.1 Implications for construction and patterns

The declaration form determines which call/pattern forms are available
for each variant:

- A variant with **named payload** supports both positional and named
  forms at construction sites and pattern matches. The choice is per-site
  per §3.5.
- A variant with **positional payload** supports only positional form at
  construction sites and pattern matches. No names were declared; named
  form is not available.

```
enum Shape:
  Circle(f64)
  Rectangle(width: f64, height: f64)

// Circle (positional declaration):
let c1 = Shape::Circle(5.0)                            // ✓ positional
let c2 = Shape::Circle(radius: 5.0)                    // ✗ no name "radius" declared

// Rectangle (named declaration):
let r1 = Shape::Rectangle(width: 10.0, height: 20.0)   // ✓ named
let r2 = Shape::Rectangle(10.0, 20.0)                  // ✓ positional (always available)
let r3 = Shape::Rectangle(width: 10.0, 20.0)           // ✗ mixed within call

// Pattern matching mirrors construction:
match shape:
  Circle(r):                                            // ✓ positional binding
    use_circle(r)
  Rectangle(w, h):                                      // ✓ positional binding
    use_rect(w, h)
  Rectangle(width: w, height: h):                       // ✓ named binding
    use_rect(w, h)
```

The form chosen at the declaration site is part of the variant's API.
Adding names to a previously positional variant is a non-breaking change
(both forms become valid); removing names from a previously named variant
is a breaking change (named-form call sites stop compiling).

##### 6.2.1.2 Choosing a form

Positional declarations are appropriate when:

- The variant has a single payload field with self-evident meaning
  (`Some(T)`, `Ok(T)`, `Err(E)`).
- The variant is conceptually a tuple with positional identity.
- Conciseness matters and the type alone documents the payload.

Named declarations are appropriate when:

- The variant has multiple payload fields whose roles aren't
  self-evident from order alone.
- The variant has multiple fields of the same type and positional order
  would be error-prone.
- Documentation value of field names outweighs the verbosity.

The stdlib uses positional payloads for `Option::Some`, `Result::Ok`, and
`Result::Err` because each carries a single value whose role is captured
by the variant name itself.

Generic parameters on the enum are in scope within all variants' payload
declarations.

#### 6.2.2 Conformance

An enum may include a `satisfies` clause listing the traits the type
implements, parallel to records:

```
enum Color:
  satisfies Display, Eq, Hash
  Red
  Green
  Blue
  Custom(r: u8, g: u8, b: u8)
```

Per §3.2, `satisfies` requires matching `fulfill` blocks. The conformance
applies to the *enum as a whole*, not per-variant. A trait implementation
for an enum handles all variants — typically via a `match` expression on
the input — and produces a uniform result type:

```
fulfill Display for Color:
  fn display(value: Color) -> string:
    match value:
      Red: "red"
      Green: "green"
      Blue: "blue"
      Custom(r, g, b): "rgb({r}, {g}, {b})"
```

#### 6.2.3 Variant construction and resolution

A variant value is constructed by naming the variant and (for payload
variants) supplying its arguments:

```
let d = Direction::North
let c = Shape::Circle(5.0)                         // positional (Circle declared positionally)
let r1 = Shape::Rectangle(width: 10.0, height: 20.0)   // named (Rectangle has names)
let r2 = Shape::Rectangle(10.0, 20.0)              // positional (always available)
let res: Result[i32, string] = Result::Ok(42)
let n: Option[i32] = Option::None
```

By default, every variant reference is *path-qualified* with the enum name
via `::` (`Result::Ok`, `Direction::North`). The path qualification makes
the variant's enum unambiguous at every use site.

Unqualified variant names are not available by default. To bring variants
into scope unqualified, the user explicitly imports them via `use`:

```
use Result::(Ok, Err)
use Direction::*

let r = Ok(42)                                 // ✓ Result::Ok imported
let e = Err("bad")                             // ✓ Result::Err imported
let d = North                                  // ✓ all Direction variants imported
```

Selection lists in `use` paths use parentheses. The language uses `()` for
grouping uniformly — function arguments, generic arguments, tuple
construction, expression grouping, trait intersection (`dyn (A & B)`) —
and path selection follows the same convention. The context disambiguates
the two uses of `()`: after `::` it is a selection list; after a value
expression it is a call.

Two enums imported into the same scope whose variants have colliding
names produce an *import-time* conflict, not a call-site ambiguity:

```
use Direction::*       // brings North, South, East, West
use Heading::*         // ERROR: Heading::North conflicts with Direction::North
```

The user resolves by importing selectively (`use Heading::(East, West)` if
only some variants don't conflict) or by importing one enum's variants
and keeping the other path-qualified.

Conflicts are surfaced where they originate (the `use` statements), not
where the offending name would be used. This keeps call sites unambiguous
and makes import-induced confusion visible at the import declarations.

#### 6.2.4 Pattern matching

The `match` expression is the canonical way to consume an enum value
(grammar §3.13's `MatchExpr`). Each arm specifies a pattern and an
expression:

```
let area = match shape:
  Circle(radius):
    f64::PI * radius * radius
  Rectangle(width, height):
    width * height
  Triangle(a, b, c):
    let s = (a + b + c) / 2.0
    (s * (s - a) * (s - b) * (s - c)).sqrt()
```

Variant patterns parallel variant construction (§6.2.1.1): they may use
*positional* form binding payload fields by declaration order, or
*named* form binding by field name (when the variant declared field
names). Mixing the two within one pattern is a compile error.

```
// Positional form — bindings in declaration order:
Rectangle(width, height): ...

// Named form — bindings by field name (requires named declaration):
Rectangle(width: w, height: h): ...

// Named form with bound name matching field name:
Rectangle(width: width, height: height): ...    // verbose; the positional form is equivalent

// Mixed — error:
Rectangle(width, height: h): ...                // ✗ compile error
```

Named-form patterns are available only when the variant was declared with
named payload fields (§6.2.1). Positionally-declared variants accept
positional patterns only — there are no field names to match. For
example, `Some(T)` (positionally declared) accepts `Some(x)` but not
`Some(value: x)`.

In the named form, the syntax `field_name: bound_name` binds the
variant's field value to a new local name. The positional form
`Rectangle(width, height)` (binding `width` and `height` as the local
names) is the conventional terse choice when the field names happen to
match the desired local names.

Patterns may be nested for compound values:

```
match (a, b):
  (Ok(x), Ok(y)): x + y
  (Ok(_), Err(e)): panic("right error: {e}")
  (Err(e), _): panic("left error: {e}")
```

Wildcard patterns (`_`) match without binding. Catch-all patterns (a bare
identifier with no constructor) match any value and bind it.

#### 6.2.5 Exhaustiveness checking

A `match` expression must be exhaustive: every possible variant of the
matched enum (and every combination, for compound matches) must be covered
by some arm. The compiler verifies exhaustiveness at compile time. A
non-exhaustive match is a compile error identifying which variants are
unreached.

Exhaustiveness is structural: adding a new variant to an enum makes every
non-exhaustive match throughout the codebase fail to compile, surfacing the
sites that need updating. This is one of the language's principal safety
properties: enums and matches are an early-warning system for evolution.

A catch-all arm (`_:` or a bare identifier) covers all remaining variants
and makes the match trivially exhaustive. Users may opt into this when
adding a new variant should be silently absorbed (rare and usually a
mistake).

#### 6.2.6 Enum visibility

Visibility per §10 applies to the enum as a whole, not per-variant:

```
public enum Direction:
  North
  South
  East
  West
```

The enum's variants share the enum's visibility. There is no per-variant
visibility specifier. If a user wants some variants visible and others
hidden, they split the enum into multiple enums (each with its own
visibility) and provide conversion functions between them.

#### 6.2.7 Trait auto-derivation

Per §3.8, enums support `@derive` for the same fixed set of traits as
records:

```
@derive(Eq, Ord, Hash, Clone, Display, Debug)
enum Color:
  Red
  Green
  Blue
  Custom(r: u8, g: u8, b: u8)
```

Derivation operates variant-by-variant. For `Eq`, the implementation
compares variant tags and, for matching tags, compares payload fields
pairwise. For `Ord`, variants are ordered by declaration order, with ties
broken by payload comparison. For `Hash`, the variant tag and payload
fields are combined. For `Clone`, each variant's payload is structurally
copied. For `Display` and `Debug`, the generated output is a
compiler-defined structural format.

Derivation requires every variant payload's field type to itself satisfy
the trait being derived. Failure is a compile error identifying the
offending payload field.

### 6.3 Newtypes

A newtype is a wrapper type that creates a new nominal identity over an
existing type. Newtypes are the standard way to add domain meaning to a
primitive or stdlib type, satisfy the orphan rule for foreign-trait +
foreign-type combinations (§3.7.5), or enforce invariants at construction.

#### 6.3.1 Declaration

A newtype is declared with the `type` keyword and a body containing a
`wraps` clause naming the underlying type:

```
type Email:
  wraps string

type UserId:
  wraps i64

type Distance:
  wraps f64

type MyVec[T]:
  wraps Vec[T]
```

The signature line matches ordinary record and enum declarations
(`type Name[generics]:`) for uniformity. The `wraps` clause inside the
body identifies the declaration as a newtype and names its underlying
type. The body may include other clauses — `satisfies` clauses or
metadata declarations — but it may not contain field declarations. A
`wraps` body and a field-declaration body are mutually exclusive: a
newtype wraps one underlying value; a record declares its own fields.
The compiler rejects bodies that mix `wraps` with field declarations.

The contrast with `alias type` from §4.2:

```
alias type byte = u8         // transparent alias; byte and u8 are the same type
type UserId:                 // newtype; UserId is distinct from i64
  wraps i64
```

`alias type` produces transparent substitution — no new identity. A
`type` declaration with a `wraps` clause produces a *new* nominal
identity. The two forms are syntactically distinct and serve opposite
purposes.

A newtype body may include `satisfies` clauses for explicitly implemented
traits per §3.2:

```
type Email:
  wraps string
  satisfies TryFrom[string]

fulfill TryFrom[string] for Email:
  type Error = ValidationError
  fn try_from(s: string) -> Result[Email, Error]:
    if is_valid_email(s):
      Ok(Email(s))
    else:
      Err(ValidationError::Invalid)
```

The same `satisfies`/`fulfill` discipline from §3.2 applies. The
`@derive` annotation per §3.8 is the shorthand for the common case where
trait conformance is structural over the underlying type.

#### 6.3.2 Construction and extraction

A newtype is constructed by calling its type name with the underlying
value as a single positional argument:

```
let email = Email("alice@example.com")
let id = UserId(42)
let distance = Distance(1.5)
```

Construction is always positional with one argument — the underlying
value. No named-argument form, no multi-argument form. The newtype wraps
exactly one value; the constructor reflects that shape.

Extraction of the underlying value uses the `as` operator:

```
let s: string = email as string      // unwraps Email to string
let n: i64 = id as i64               // unwraps UserId to i64
let d: f64 = distance as f64         // unwraps Distance to f64
```

##### Note on `as` overloading

The `as` operator has two distinct uses in the language, dispatched by
operand kind:

- **Numeric cast** (§4.7) — converts between numeric primitive types,
  potentially with trap-on-range-violation. Both operand and target are
  numeric primitives.
- **Newtype extraction** (here) — unwraps a newtype to its underlying
  type. The operand is a newtype; the target is its `wraps` type.

The two uses are unambiguous because the operand types determine the
applicable mode: `5_i32 as f64` is a numeric cast (both are numeric
primitives); `email as string` is a newtype extraction (`Email` is a
newtype, `string` is its wrapped type). Mixing — e.g., extracting and
re-casting in one operation — requires two `as` applications:

```
let n_str: string = some_userid as i64 as string  // ERROR: i64 -> string isn't a numeric cast
let n: i64 = some_userid as i64
let s = n.to_string()                              // use stdlib conversion
```

The asymmetric construction/extraction interfaces are deliberate.
Construction is a *creation* of new identity (typed call). Extraction is
a *discarding* of identity (explicit cast). The two operations are kept
syntactically distinct so that a reader sees clearly when domain identity
is being introduced versus removed.

#### 6.3.3 Trait inheritance via `@derive`

By default, a newtype inherits *no* traits from its underlying type. The
nominal-identity-creating purpose of a newtype is undermined if it
automatically inherits behavior — users typically introduce a newtype
precisely to *not* expose the underlying type's operations.

Trait inheritance is opt-in via `@derive`:

```
@derive(Eq, Hash, Display)
type Email:
  wraps string

@derive(Add, Sub, Mul)
type Distance:
  wraps f64

@derive(Eq, Ord, Clone)
type UserId:
  wraps i64
```

For each derived trait, the compiler generates a `fulfill` block that
delegates to the underlying type's implementation. Operations on the
newtype dispatch through this delegation to the underlying behavior. For
example, `@derive(Add)` on `Distance` allows `Distance(1.0) +
Distance(2.0)` to dispatch to `f64`'s `Add::add`, producing
`Distance(3.0)`.

Operators across different newtype identities require explicit
implementation: `Distance + i32` is a compile error unless the user
writes a `fulfill Add[i32] for Distance` block manually (with a matching
`satisfies Add[i32]` in `Distance`'s body). The orphan rule (§3.7)
permits this in the newtype's defining module.

The `@derive` annotation implicitly declares `satisfies` for the listed
traits — the user does not write `satisfies Eq, Hash, Display` separately
when using `@derive(Eq, Hash, Display)`. Manual `fulfill` blocks still
require their corresponding `satisfies` clauses in the body per §3.2.

Derivation fails (compile error) if the underlying type does not satisfy
the trait being derived — `@derive(Display)` on a newtype wrapping a
non-`Display` type is rejected at the annotation site.

Deriving `Ord` requires the underlying type to satisfy `Eq`, parallel to
records (§6.1.8).

#### 6.3.4 Constructor visibility

Like records (§6.1.7), a newtype's constructor visibility is
independently controllable from its type visibility:

```
public(private) type Email:
  wraps string
  satisfies TryFrom[string]
```

This is the smart-constructor pattern: the type name `Email` is visible
publicly (so other modules can use `Email` in signatures), but
construction `Email(...)` is restricted (so only the defining module can
produce `Email` values, typically via a validating `From[string]` or
`TryFrom[string]` impl that enforces invariants).

The pattern is the language's mechanism for enforcing invariants at
construction time: any path that produces an `Email` value passes through
the constructor's visibility scope, which can enforce arbitrary checks.

#### 6.3.5 Newtypes and the orphan rule

A common use of newtypes is to work around the orphan rule (§3.7.1).
Implementing a foreign trait for a foreign type is forbidden, but
implementing a foreign trait for a *local newtype wrapping* the foreign
type is permitted:

```
// In user module:
type MyVec[T]:
  wraps Vec[T]
  satisfies SomeForeignTrait

fulfill SomeForeignTrait for MyVec[T]:
  ...
```

`MyVec` is local to the user's module; the orphan rule's "trait or type
defined locally" check is satisfied. The wrapping is structurally trivial
but semantically meaningful: it creates a distinct identity over which
the user has implementation authority.

---

## 7. Conversion System

User-defined conversions between types use a pair of trait pairs:
`From`/`Into` for infallible conversions and `TryFrom`/`TryInto` for
fallible conversions. The conversion system is layered on top of the trait
system (§3) and complements the built-in numeric implicit-widening rules
(§4.5) and the `as` operator (§4.7).

### 7.1 The Four Traits

```
trait From[T]:
  fn from(value: T) -> Self

trait Into[T]:
  fn into(value: Self) -> T

trait TryFrom[T]:
  type Error
  fn try_from(value: T) -> Result[Self, Error]

trait TryInto[T]:
  type Error
  fn try_into(value: Self) -> Result[T, Error]
```

`From` and `Into` describe the same conversion from two perspectives —
"construct `Self` from a `T`" vs "convert `Self` into a `T`." Likewise
`TryFrom` and `TryInto` describe the same fallible conversion.

The fallibility split is semantic. `From`/`Into` is for conversions that
cannot fail — widening, identity, lossless transformations.
`TryFrom`/`TryInto` is for conversions that can fail — narrowing, parsing,
range checks, validation. The trait the user implements signals fallibility
to every caller. Each fallible conversion declares its own `Error`
associated type, so different conversions can produce different error
kinds (range error, parse error, validation error, etc.).

### 7.2 Users Implement `From` and `TryFrom`; the Reverses Auto-Derive

`Into` and `TryInto` are *sealed* traits — declared by the language for
use in trait bounds and method dispatch, but not implementable by users.

Users write `fulfill From[T] for U` (or `fulfill TryFrom[T] for U`); the
language automatically provides the reverse direction:

- Whenever `From[T] for U` exists, `Into[U] for T` is auto-provided.
- Whenever `TryFrom[T] for U` exists, `TryInto[U] for T` is auto-provided
  with the same `Error` associated type.

The auto-derivation is language-built-in and not user-overridable. This
forecloses the coherence problem of disagreeing manual `From`/`Into` pairs.

All `Into[U] for T` impls come from auto-derivation of a corresponding
`From[T] for U` impl (plus the identity case per §7.3); all `TryInto[U]
for T` impls come from auto-derivation of `TryFrom[T] for U`. Users do
not write `fulfill Into[U] for T` or `fulfill TryInto[U] for T` directly
— the compiler synthesizes the impl from the corresponding `From` or
`TryFrom`. To expose a conversion from `T` to `U` to users, write the
`From[T] for U` impl on the destination type; the `Into` direction
follows automatically.

The `From`/`TryFrom` impls are the user's written contract; the
`Into`/`TryInto` impls are the language's mechanical counterparts.
Neither auto-derived impl requires a `satisfies` declaration on its
source type (per §3.7.3 — language-privileged implementations).

### 7.3 Identity Conversion

The language auto-implements `From[T] for T` for every type, providing the
identity conversion. The corresponding `Into[T] for T` is also auto-derived.

This makes generic code cleaner: a function parameter `T: Into[U]`
accepts both `U` (via identity) and any type explicitly convertible to
`U`. The user can pass the destination type directly without an
intermediate conversion call.

Identity conversion is not subject to the orphan rule (§3.7.3); it is one
of the language-privileged implementations.

### 7.4 The Orphan Rule Applies to User Conversions

User-written `fulfill From[T] for U` and `fulfill TryFrom[T] for U` are
subject to the standard orphan rule per §3.7.1, including the
generic-parameter-coverage rule from §3.7.2: at least one concrete local
type must appear in the impl declaration, in either the source type `T`
(the trait's argument) or the destination type `U` (the for-type).

Permitted:

```
fulfill From[i64] for MyMeasurement       // U is local ✓
fulfill From[MyMeasurement] for i64       // T is local (covers via §3.7.2) ✓
fulfill From[Vec[MyType]] for SomeType    // MyType is local, covering ✓
```

Rejected:

```
fulfill From[i64] for f64                  // ✗ neither type local — orphan
                                           //   (and language already provides this)
fulfill From[string] for Vec[i32]          // ✗ both string and Vec[i32] are foreign
```

The generic-parameter-coverage rule is particularly useful for conversions
*from* a user's type *to* a foreign type. A user owning `MyMeasurement`
can write `fulfill From[MyMeasurement] for i64` to define how their
measurement converts to a plain integer. The corresponding
`Into[i64] for MyMeasurement` auto-derives per §7.2.

For implementing a conversion between two foreign types — a relatively
rare need — the newtype pattern per §6.3.5 is the workaround: wrap one
of the foreign types in a local newtype, then implement the conversion
involving the newtype.

The auto-derivation of `Into` from `From` per §7.2 propagates this
constraint: the synthesized `Into[U] for T` impl exists at the same
module where the corresponding `From[T] for U` exists, and is bound by
the same orphan rule.

### 7.5 Built-in Numeric Conversions

The language pre-populates the conversion traits with built-in numeric
conversions per §4.5's lossless rules:

**`From` impls** (infallible) cover all lossless widening:

- Integer-to-wider-same-signedness (`i8` → `i32`, `u16` → `u64`, etc.).
- Unsigned-to-wider-signed (`u8` → `i16`, etc.).
- Float-to-wider-float (`f32` → `f64`).
- Integer-to-float for exact-representable cases (`i8`/`u8`/`i16`/`u16`
  → `f32`; `i32`/`u32` → `f64`).
- The §4.5.4 pragmatic exception: `From[i64] for f64` and
  `From[u64] for f64`.

**`TryFrom` impls** (fallible) cover narrowing, signed/unsigned crossings,
and lossy integer-to-float conversions. Each carries an appropriate
`Error` type (typically a numeric range error).

These built-in impls are language-privileged (§3.7.3) — they exist outside
user-writable `fulfill`-block space and cannot conflict with user code.

### 7.6 Relationship to `as`

The `as` operator (§4.7 for numeric, §6.3.2 for newtypes) is distinct
from the conversion-trait system but interacts with it for numeric cases:

- For **lossless numeric conversions**, `x as U` and `x.into::[U]()` (or
  equivalently `Into::into(x)` typed to `U`) produce the same result.
  Both are valid; users pick based on style. `as` is more concise; `.into()`
  is more uniform with user-defined conversions.
- For **lossy numeric conversions** that would overflow, `as` traps at
  runtime per §4.6.1; `as%` wraps; `as|` saturates; `as?` returns
  `Option[T]`. The fallible variant `try_into` returns
  `Result[T, Error]` for explicit handling with a typed error. The
  variants of `as` and `try_into` differ in what they signal: `as` and
  its variants express *value-level* range mismatches via the chosen
  policy (trap, wrap, clamp, optional); `try_into` expresses
  *trait-level* fallibility with a named `Error` type.
- For **newtype extraction**, `as` is the dedicated unwrap operation
  (§6.3.2). The conversion-trait system does not participate; the
  underlying value is exposed via the operator directly.
- For **user-defined conversions on non-newtype types**, `as` is not
  available. Users use `.into()`, `From::from()`, or `.try_into()` per
  §7.8.

The summary: `as` (with its variants) is the operator for built-in
numeric casts and newtype unwraps; the conversion traits are the
mechanism for everything else.

### 7.7 No Implicit User-Defined Conversions

User-defined `From` impls do *not* produce implicit conversions. The
implicit-conversion surface of the language is strictly limited to the
built-in lossless widenings specified in §4.5. A user implementing
`From[Celsius] for Fahrenheit` does not enable `let f: Fahrenheit = some_c`
without explicit invocation; the user writes `let f: Fahrenheit =
some_c.into()` or `let f: Fahrenheit = Fahrenheit::from(some_c)`.

This prevents the C-family hazard of action at a distance through
user-defined conversions. The set of types that auto-convert is fixed by
the language and discoverable from §4.5; user types never silently
participate in expression-level type adjustment.

The auto-derivation of `Into` from `From` (§7.2) is *not* an implicit
conversion — it is the auto-generation of a callable trait method.
Calling that method requires explicit syntax at the call site, dispatched
through uniform call syntax (§3.4).

### 7.8 Invocation Forms

Conversion calls use the standard uniform call syntax per §3.4 and follow
the argument-form rules per §3.5. Three explicit forms are available
universally; a fourth implicit form applies only to built-in lossless
widenings.

```
let x: f64 = (5_i32).into::[f64]()        // method form
let x: f64 = Into::into(5_i32)            // free-function via trait path
let x: f64 = From::from(5_i32)            // free-function via trait path
let x: f64 = 5_i32                        // implicit (built-in lossless widening only)
```

The first three forms are explicit invocations and are available for all
`From`/`Into` impls — built-in and user-defined alike. The fourth is not
an invocation at all but the absence of one: it works only because
`i32` → `f64` is in the built-in lossless-widening set (§4.5.2), where
the compiler inserts the conversion silently. User-defined `From` impls
never participate in implicit conversion (§7.7).

The trait-path free-function form `Trait::method(value)` works with
generic trait methods (like `Into::into`) when the target type can be
inferred from context — typically from an annotation on the binding
(`let x: f64 = ...`) or from a downstream constraint. When inference
isn't sufficient, the method form with explicit turbofish
(`x.into::[U]()`) is the clearer choice.

Fallible conversions return `Result[T, Error]` and typically chain through
the `?` operator (§8) for propagation:

```
let r: Result[i32, _] = big_value.try_into::[i32]()
fn parse_age(s: string) -> Result[Age, ParseError]:
  let n: i32 = s.parse::[i32]()?
  let age: Age = n.try_into::[Age]()?
  Ok(age)
```

The `?` operator's interaction with `From` for error-type conversion is
specified in §8 (and constrained per §7.9).

### 7.9 Error-Type Relationships in `?` Propagation

The `?` operator (§8) extracts the success value from a `Result` or
`Option`-typed expression. On failure, it propagates the failure value up
the call stack — terminating the current function early with a converted
failure if needed.

For propagation to succeed, the source's *error type* must be the same as
the destination function's error type, or be convertible to it via `From`:

```
fn parse_to_string(s: string) -> Result[string, ParseError]:
  let n: i32 = s.parse::[i32]()?      // source: Result[i32, ParseError]
                                       //   error types match: ParseError = ParseError ✓
  Ok(n.to_string())                    // function returns Result[string, ParseError]

fn read_and_parse(path: string) -> Result[i32, AppError]:
  let bytes: Vec[u8] = read_file(path)?   // source: Result[Vec[u8], IoError]
                                          //   IoError → AppError via From: ✓
  let s: string = parse_string(bytes)?     // source: Result[string, ParseError]
                                          //   ParseError → AppError via From: ✓
  let n: i32 = s.parse::[i32]()?
  Ok(n)
```

The success type at the `?` site becomes the type of the expression at
that site, bound to the local variable on the left or used inline.
Different `?` sites in the same function can produce different success
types — `?` does not impose any constraint between the source's success
type and the function's return success type. That contract is satisfied
separately, wherever the function actually returns `Ok(...)`.

The error-type rule:

- **Same error type:** trivially valid; no conversion.
- **Source error convertible to destination error via `From`:** the
  compiler inserts the `From::from` call automatically at the propagation
  site.
- **No relationship via `From`:** compile error at the `?` site,
  identifying the source and destination error types and the missing
  `From` impl.

This rule is the *only* relationship `?` enforces between source and
destination types. There is no implicit success-type coercion, no
fallback through arbitrary trait machinery, no silent type adjustment.
The `From`-bound error conversion is opt-in by the user (via implementing
`From[SourceError] for DestError`); without it, `?` is a hard type error.

This bounded model gives `?` predictable behavior: a reader sees `?` and
knows exactly two things — "extract success here; propagate error
upward, converting via `From` if the types differ." Anything more
elaborate happens through explicit `match` or method chains.

---

## 8. Error Handling

The language uses a two-track failure model. The distinction is made *at
the operation site* when writing code; once a failure has been encoded as
one kind, it cannot be silently converted to the other.

### 8.1 The Two-Track Model

**Trap-track failures** represent bugs and invariant violations:
arithmetic overflow on default operators per §4.6.1, integer division by
zero, out-of-range `as` casts, out-of-range array indices, `abs` on
signed minimum (§4.8), negative integer exponent on integer base
(§4.8.3), `unwrap`/`expect` on `Option::None` or `Result::Err`, runtime
stack overflow, allocation failure, and explicit `panic` calls. Traps
halt execution and produce diagnostics. They are *not* catchable as
values.

Non-exhaustive `match` expressions are a separate concern: they are
*compile errors* per §6.2.5, not runtime traps. The compiler statically
verifies exhaustiveness at every `match`; a non-exhaustive match never
compiles. If the user wants a runtime panic for "unreachable" cases,
they write an explicit catch-all arm calling `panic` (which produces a
trap via the standard mechanism).

**Value-track failures** represent recoverable conditions that flow
through the type system: `Option[T]` for failures carrying no
information beyond their occurrence, `Result[T, E]` for failures
carrying contextual information, the `?` operator for short-circuit
propagation (§8.4), the `Try` trait dispatching `?` to
user-implementable types, the `From`-conversion of failure types during
propagation (§7.9), and the arithmetic operator variants (`+?`, `-?`,
etc.) per §4.6.4 for producing `Option`-typed results from operations
that would otherwise trap.

The two tracks are not interchangeable:

- A trap does not become a `Result::Err` value.
- A `Result::Err` does not abort the program.
- There is no `try`/`catch` mechanism for traps.

The user picks the mechanism based on the failure's nature when writing
the code: traps for "this should never happen if the program is correct";
`Option`/`Result` for "this might legitimately happen at runtime and the
caller might want to handle it." The operator variants from §4.6 make
this choice visible at the operation level itself: `+` traps on overflow
(the "if this overflows, the program has a bug" choice); `+?` returns
`Option[T]` (the "the caller wants to handle the overflow case" choice).

### 8.2 The Trap Track

#### 8.2.1 `panic` and the `never` type

`panic` is a built-in function in the language prelude — available
without qualification in every scope. It has the signature:

```
fn panic(message: string) -> never
```

It triggers an immediate trap with the given diagnostic message. The
`never` return type allows `panic` to appear anywhere a value of any
type is expected, including inside `match` arms, conditional branches,
and function bodies that return non-unit types:

```
let value = match maybe_value:
  Some(x): x
  None: panic("expected Some, got None")
```

#### 8.2.2 The `never` type

`never` is a built-in primitive type with no values, written in lowercase
per the convention for primitive type keywords (§1.4). It is the return
type of functions that do not return normally — `panic`, infinite loops,
functions that always trap.

The compiler treats `never` as unifiable with any type during
type-checking: a value of type `never` can be used in any context
expecting any other type, because such a value can never actually exist
at runtime. This is the "bottom type" of type theory, exposed as an
ordinary primitive.

```
fn unreachable() -> never:
  panic("unreachable code reached")

let x: i32 = if condition: 5 else: unreachable()
                                    // unreachable() returns never;
                                    // unifies with i32 ✓
```

#### 8.2.3 Trap behavior at runtime

When a trap fires:

1. A diagnostic is printed including the operation that triggered the
   trap (with operand values where available), the source location
   (file, line, column), and a stack trace through the call chain.
2. The process exits.

There is no recovery mechanism. No `try`/`catch` exists for traps. No
unwinding hook can intercept and convert a trap to a value. The
philosophy: a trap signals a bug; the program is in a state the
programmer didn't anticipate; continuing risks further incorrect
behavior. Process abort is the safe response.

Once a trap fires, the program exits. The only way to handle a failure
recoverably is to use `Result`/`Option` from the start — and the
operator/method variants in §4.6 and §4.7 (e.g., `+?`, `as?`,
`checked_div`) make this choice available where overflow or range
violation is a possibility. The user decides at the operation site
whether a failure is a bug to be trapped or a condition to be handled.
Choosing wrong at that point cannot be retroactively patched by a
`catch` block; the language forces the decision upfront, which is the
principal mechanism for keeping the two failure tracks honest.

#### 8.2.4 Diagnostic format

The diagnostic format includes the operation name and operand values
where the runtime has access to them:

```
panic: integer overflow: 2147483647 + 1
  at compute_total, src/billing.duc:42:8
  called from main, src/main.duc:7:3
```

For user-triggered `panic` calls, the diagnostic includes the
user-supplied message:

```
panic: expected Some, got None
  at process_input, src/handler.duc:24:10
```

Format details are implementation-level. The semantic commitment is that
diagnostics provide sufficient information to identify what trapped,
where, and through what call chain.

### 8.3 The Value Track: `Option` and `Result`

`Option[T]` and `Result[T, E]` are standard library types built from the
generic enum mechanism per §6.2. They are ordinary enums with no
language-level special-casing of their identity. Their stdlib
definitions:

```
enum Option[T]:
  Some(T)
  None

enum Result[T, E]:
  Ok(T)
  Err(E)
```

The interactions that look special — the `?` operator (§8.4), the
error-conversion chains via `From` (§8.5) — are mediated through a
stdlib trait (`Try`), not through compiler knowledge of these specific
types. Any user-defined type can participate in `?` propagation by
implementing `Try` per §8.4.

#### 8.3.1 Pattern matching

`Option` and `Result` use standard exhaustive `match` per §6.2.4:

```
match maybe_value:
  Some(x): use_it(x)
  None: handle_absence()

match operation:
  Ok(value): proceed(value)
  Err(error): handle_error(error)
```

No special `if let` or check-and-unwrap sugar is provided in v1. The
combination of `match` (for full discrimination) and `?` (for
short-circuit propagation) covers the common cases. The language
surface is intentionally minimal; sugar may be added later if usage
patterns reveal a sharp need.

### 8.4 The `?` Operator and the `Try` Trait

The `?` postfix operator (grammar §3.15) dispatches through a stdlib
trait, `Try`, that decomposes a value into either a "continue with this
success value" or "break with this failure value":

```
trait Try:
  type Success
  type Failure
  fn branch(value: Self) -> TryBranch[Success, Failure]

enum TryBranch[S, F]:
  Continue(S)
  Break(F)
```

`Option` and `Result` fulfill `Try` in stdlib:

- `Try::branch(Some(x))` → `Continue(x)`; `Try::branch(None)` →
  `Break(())`.
- `Try::branch(Ok(x))` → `Continue(x)`; `Try::branch(Err(e))` →
  `Break(e)`.

For `Option[T]`, `Failure = ()` (unit) — there is no inner error value
beyond the absence itself. For `Result[T, E]`, `Failure = E` — the error
value. The desugaring in §8.4.1 applies `From::from` to this inner
failure value, not to the wrapped `None`/`Err(...)` container.

User types may implement `Try` to make `?` available on their own
optional-or-result-like types.

#### 8.4.1 Desugaring

The `?` operator desugars to a `match` on the trait method's result,
with the failure branch returning from the enclosing function and
applying `From`-conversion to bridge failure types:

```
expr?
```

desugars to:

```
match Try::branch(expr):
  Continue(value): value
  Break(failure): return From::from(failure)
```

The `From::from(failure)` automatically converts the failure value into
the enclosing function's failure type. When the failure types are
identical, `From::from` is the trivial identity conversion (§7.3); no
special-case logic is needed for matching types.

Under this desugaring, `From::from(failure)` converts the inner failure
value to the enclosing function's error/absence type. For Result-to-Result
propagation, this is the user's `From[SourceError] for DestError` impl
(§7.9). For Option-to-Option propagation, the auto-implementation
`From[()] for Option[T]` (yielding `None`) is provided by stdlib.
Cross-type propagation (Option in a Result-returning function, or vice
versa) remains forbidden per §8.6 — the failure types are not compatible.

### 8.5 Error-Type Conversion via `From`

The `From::from(failure)` step in `?` propagation enables error-type
chains: a function returning `Result[T, MyError]` can use `?` on any
`Result[U, OtherError]` provided `fulfill From[OtherError] for MyError`
exists. The conversion is invisible at the call site but typed
end-to-end; the compiler verifies the `From` impl exists at every `?`
use site, rejecting with a clear error when no path is found.

Full rules for the error-type relationship are specified in §7.9. In
brief:

- Same error type: trivially valid.
- Source error convertible to destination error via `From`: implicit
  conversion at the propagation site.
- No relationship via `From`: compile error at the `?` site.

The success type at the `?` site becomes the local expression's value
and has no relationship to the function's return success type — the
function's `Ok(...)` site satisfies that contract separately.

### 8.6 No Cross-Type `?`

Using `?` on an `Option` value inside a function returning `Result`, or
on a `Result` value inside a function returning `Option`, is a compile
error. This is a categorical rule enforced by the compiler at every
`?` site — regardless of whether `From` impls exist that could in
principle bridge the failure types (`From[()] for SomeError` or
`From[SomeError] for ()`).

The rule's justification: `Option`'s `None` carries no information,
while `Result`'s `Err` carries an error value. Silently bridging them
would require either fabricating an error value from `None` (what
information?) or discarding an error value when going to `Option`
(information loss without user signal). Both cross-category bridges
are operations that should be explicit at the call site, never
implicit through `?`.

The user converts explicitly via stdlib methods (§8.7): `option.ok_or(err)`
produces `Result[T, E]` from `Option[T]` with an explicit error value;
`result.ok()` produces `Option[T]` from `Result[T, E]`, discarding the
error.

### 8.7 Standard Methods

Stdlib provides a standard set of methods on `Option` and `Result`.
The non-exhaustive list:

#### 8.7.1 `Option[T]`

- `unwrap(value: Self) -> T` — returns the value or traps if `None`.
- `expect(value: Self, msg: string) -> T` — like `unwrap` with custom
  trap message.
- `unwrap_or(value: Self, default: T) -> T` — returns the value or the
  default.
- `unwrap_or_else(value: Self, f: fn() -> T) -> T` — returns the value
  or a computed default.
- `map[U](value: Self, f: fn(T) -> U) -> Option[U]` — applies a
  function to the success value.
- `and_then[U](value: Self, f: fn(T) -> Option[U]) -> Option[U]` —
  chains optional computations.
- `or_else(value: Self, f: fn() -> Option[T]) -> Option[T]` — fallback
  computation.
- `ok_or[E](value: Self, err: E) -> Result[T, E]` — converts to
  `Result` with the given error on `None`.
- `is_some(value: &Self) -> bool`, `is_none(value: &Self) -> bool` —
  discriminator predicates.

#### 8.7.2 `Result[T, E]`

- `unwrap(value: Self) -> T` — returns success or traps on `Err`.
- `expect(value: Self, msg: string) -> T` — like `unwrap` with custom
  trap message.
- `unwrap_or(value: Self, default: T) -> T`,
  `unwrap_or_else(value: Self, f: fn(E) -> T) -> T`.
- `map[U](value: Self, f: fn(T) -> U) -> Result[U, E]` — transforms the
  success value.
- `map_err[F](value: Self, f: fn(E) -> F) -> Result[T, F]` — converts
  the error type.
- `and_then[U](value: Self, f: fn(T) -> Result[U, E]) -> Result[U, E]`
  — chains fallible computations.
- `or_else[F](value: Self, f: fn(E) -> Result[T, F]) -> Result[T, F]`
  — error-recovery chain.
- `ok(value: Self) -> Option[T]`, `err(value: Self) -> Option[E]` —
  convert to `Option`, discarding the other arm.
- `is_ok(value: &Self) -> bool`, `is_err(value: &Self) -> bool` —
  discriminator predicates.

All methods listed above are *free functions* defined in stdlib, callable
through uniform call syntax per §3.4 (records and enums carry no methods
of their own per §6.1.9 and §6.2.6). The following are equivalent:

```
option.unwrap()
unwrap(option)
std::option::unwrap(option)        // module-path qualification
```

The module-path form `std::option::unwrap(option)` is used to
disambiguate when multiple `unwrap` free functions are in scope (e.g.,
the `unwrap` in `std::option` and the `unwrap` in `std::result`). Path
qualification follows the module-path rules in §10. There is no
`Option::unwrap(option)` (type-qualified) form: free functions live in
modules per §10, not associated with types, and the dispatch model in
§3.4 does not include a type-qualified free-function namespace.

Note on closure-type notation: the `fn(T) -> U` parameter types shown
in the method signatures use a forward-referencing notation for closure
types. The complete closure-type specification is deferred to a future
spec revision. For v1, treat these signatures as taking any callable
value (free function, closure, or operator-applied function) whose
call-arity and parameter/return types match.

### 8.8 Convention: `Option` vs `Result`

The choice between `Option` and `Result` is convention, not a language
rule:

- Use `Option[T]` when the failure case carries no information beyond
  its occurrence (e.g., `find_first(predicate)` — the element exists or
  it doesn't; there's nothing more to say).
- Use `Result[T, E]` when the failure case carries information the
  caller may want to inspect or react to (e.g., `read_file(path)` —
  the caller often wants to know whether the failure was missing-file,
  permission-denied, or transient I/O error).

When in doubt, prefer `Result`. Information about failure is rarely too
much; the absence of information makes debugging harder. The compiler
accepts either signature; users choose based on what callers need.

### 8.9 Error Handling in the Reactive Context

The reactive system (specified in §13.2.3 for derived error
semantics) uses the same two-track failure model. A trap inside a `derived` expression's
computation propagates as a normal trap — the reactive system does not
catch traps. A `derived` declaration whose expression has type
`Result[T, E]` or `Option[T]` produces a reactive value of that type;
consumers of the derived value handle the failure case using standard
`match` or `?` propagation. The reactive layer adds no special error
mechanism beyond what already exists in the type system.

---

## 9. Strings, Tuples, and Arrays

This section specifies three foundational compound types that are
not user-defined: `string` (a primitive built-in), tuples (structural
anonymous products), and fixed-size arrays (`T[N]`). All three have
dedicated syntax and language-level treatment; their behaviors are
specified here rather than emerging from the trait system alone.

### 9.1 Strings

`string` is a built-in primitive type, at the same level as `i32` or
`bool`. The compiler has direct knowledge of it; it is not a stdlib type
with privileged literal syntax. The built-in status enables compiler-level
optimizations (small-string optimization, intern pools, constant folding
of string literals per §2.4) without dependency on a stdlib
implementation. The lowercase `string` keyword is reserved, matching the
lowercase convention for primitive types (§1.4).

#### 9.1.1 Primitive non-numeric types

The complete set of primitive non-numeric types in the language is:

- `bool` — the truth-value type.
- `char` — a Unicode scalar value (see §9.1.2).
- `string` — UTF-8-encoded sequences of `char` values (see §9.1.3 onward).
- `duration` — a span of monotonic time (see §9.4.1).
- `instant` — a point in monotonic time (see §9.4.2).

No other non-numeric primitives exist. Byte sequences are `u8[N]` arrays
(§9.3). Other text-related types (UTF-16 strings, ASCII-only strings,
byte strings with no encoding) are stdlib concerns if needed; the language
commits to one string type, and that type is UTF-8. Wall-clock dates,
calendar arithmetic, and timezones are stdlib concerns; the language
commits to monotonic time only.

#### 9.1.2 The `char` type

`char` represents a Unicode scalar value — an integer in the range
`0..=0xD7FF` ∪ `0xE000..=0x10FFFF`. The excluded range
(`0xD800..=0xDFFF`) is reserved for UTF-16 surrogate pairs and is not a
valid scalar value. A `char` value is always a valid Unicode scalar; the
type system rejects values outside this range at construction time.

Representation is 32-bit per value (`char` does not vary in size despite
representing a code-point range that fits in 21 bits — fixed width
enables direct indexing of `char` sequences).

**Character literals** use single quotes:

```
let c1: char = 'a'
let c2: char = '\n'                 // newline
let c3: char = '\t'                 // tab
let c4: char = '\\'                 // literal backslash
let c5: char = '\''                 // literal single quote
let c6: char = '\u{1F600}'          // 😀  (escape for any Unicode scalar)
let c7: char = '\x41'               // 'A' (escape for ASCII byte)
```

The same escape conventions as string literals (§9.1.3) apply. A
character literal contains exactly one Unicode scalar; multi-character
literals are a compile error.

**Conversion with integers** uses the conversion-trait system per §7:

- `From[char] for u32` — every `char` converts to a `u32` losslessly
  (a Unicode scalar fits in 21 bits, well within u32's range).
- `TryFrom[u32] for char` — only valid Unicode scalar values produce
  a `char`; surrogate-pair range and values above `0x10FFFF` produce
  `Err`.

`char` is `Eq`, `Ord`, `Hash`, `Display`, `Debug`, and `Clone` — the
standard trait set for primitive scalar types. Comparison and ordering
follow numeric Unicode scalar value order.

**Relationship to strings**: A `string` is conceptually a sequence of
`char` values encoded as UTF-8. The `chars()` view (§9.1.6) produces a
`char` sequence; the `chars` method's complexity is O(n) because UTF-8
decoding is required to extract each `char`.

#### 9.1.3 String literals

String literals follow grammar §2.5.5:

- **Plain strings**: `"hello world"`.
- **Raw strings**: `r"no \n escapes"`, `r#"with "quotes""#`.
- **Escape sequences**: `\n`, `\t`, `\\`, `\"`, `\xHH`, `\u{HHHHHH}`.
- **Interpolation**: `"user {name} has {count} items"`.

All forms produce values of type `string`.

#### 9.1.4 UTF-8 invariant

UTF-8 is the internal encoding. Strings are sequences of bytes
interpretable as UTF-8; the type system guarantees that every string
value is valid UTF-8. No invalid-UTF-8 string can exist at runtime;
constructors and conversions that take untrusted input either reject
ill-formed input or return a fallible result.

#### 9.1.5 No direct indexing

Strings are opaque with respect to indexing — there is no `s[i]`
operator. Direct indexing is rejected as a footgun:

- Byte indexing produces meaningless results when an index lands
  mid-codepoint in a multi-byte UTF-8 sequence.
- Character indexing is O(n) (since UTF-8 is variable-width) and would
  silently hide that cost behind constant-time-looking syntax.
- Both invite subtle bugs that only surface on non-ASCII input.

Access to string contents requires explicit views per §9.1.6.

#### 9.1.6 Views and queries

Access to string contents uses explicit methods that make the unit of
measurement visible at every call site:

- `s.bytes()` — returns a sequence of `u8` values representing the
  raw bytes. Indexable in O(1), but the user is responsible for
  UTF-8-aware handling of multi-byte sequences. The exact return type is
  a stdlib concern.
- `s.chars()` — returns a sequence of `char` values (Unicode scalars).
  Iterable in O(n) total traversal. The exact return type is a stdlib
  concern.
- `s.byte_len() -> isize` — length in bytes. O(1).
- `s.char_count() -> isize` — number of Unicode scalars. O(n).

Each name describes both the operation and its complexity-relevant unit.
Users choose the appropriate view for their workload; the language does
not pick a default that would be wrong for some cases.

#### 9.1.7 Slicing

Slicing uses explicit methods rather than range syntax:

- `s.slice(start: isize, end: isize) -> string` — char-boundary slicing.
  `start` and `end` are character positions. Boundaries are validated.
  Cost is O(end) — char boundaries are located by walking UTF-8 from the
  start, since UTF-8 is variable-width.
- `s.byte_slice(start: isize, end: isize) -> string` — byte-boundary
  slicing. `start` and `end` are byte positions. Traps if a boundary
  lands mid-codepoint (which would produce invalid UTF-8). Cost is
  O(1) for boundary lookup; the validation requires reading the byte at
  each boundary to verify it does not fall inside a multi-byte sequence,
  still O(1) per boundary.

Both methods return a new string value. Invalid boundaries
(mid-codepoint byte index, out-of-range positions) trap at runtime per
§4.6.1's trap-on-error philosophy.

#### 9.1.8 Immutability and operations

Strings are immutable, consistent with all bindings in the language per
§1.3. There is no in-place mutation. Every string operation that
produces modified content returns a new string value:

```
let upper = s.to_upper()
let trimmed = s.trim()
let replaced = s.replace(old, new)
let combined = a + b
```

The runtime is free to share immutable backing storage between values,
but this is an implementation detail invisible to the user.

The `+` operator concatenates strings per §4.4's operator framework.
The language provides an `Add` implementation for `string` (per §3.7.3's
language-privileged impls) with both operands and result typed as
`string`:

```
let greeting = "hello" + " " + "world"
```

#### 9.1.9 Interpolation

Interpolation is the preferred form when building strings from
non-string values, per grammar §2.5.5:

```
let label = "user {name} has {count} items"
let summary = "value: {amount * tax_rate}"
```

The interpolation expression `{expr}` evaluates the expression and
converts the result to `string` via the `Display` trait per §3.7. Values
whose types do not satisfy `Display` produce a compile error at the
interpolation site.

Interpolation expressions are arbitrary expressions, including method
calls, arithmetic, and field access. They are not limited to bare
identifiers.

### 9.2 Tuples

Tuples are *structurally typed* — the one structural-typing carve-out in
an otherwise nominal type system. Two tuples with the same component
types in the same order are the same type:

```
(1, 2)         // (i32, i32)
(3, 4)         // also (i32, i32) — same type as above
(1, "hello")   // (i32, string) — a different type
```

No type declaration is required to use a tuple type; the type expression
`(T1, T2, ...)` denotes the tuple type directly. The structural-typing
carve-out is justified by the fact that tuples are anonymous product
types by design and carry no domain identity — there is no nominal
contract to preserve.

#### 9.2.1 Field access

Field access uses numeric postfix syntax per grammar §3.15:

```
let t = (1, "hello", 3.14)
let n = t.0          // i32
let s = t.1          // string
let f = t.2          // f64
```

Indices are zero-based and must be **integer literals**. Bounds checking
happens at compile time: `t.3` on a 3-tuple is a compile error.

The literal restriction is structural: tuple components can have
different types, and the compiler must know the type of the accessed
field statically. Runtime indexing with a variable expression (`t.i`
where `i` is a binding) is not permitted because the type of the result
would depend on a runtime value, which the type system cannot express.

#### 9.2.2 Pattern destructuring

Tuple patterns follow grammar §3.14's `TuplePat`:

```
let (a, b, c) = (1, "hello", 3.14)
let (x, _, z) = some_tuple
let ((a, b), c) = ((1, 2), 3)
```

Tuple patterns appear in `let` bindings, `match` arms, and any other
position where patterns are admitted. Nested tuple patterns work to
arbitrary depth. The wildcard `_` ignores a component without binding it.

Tuple patterns are always positional per §3.5 — tuples have no field
names, so there is no named form.

#### 9.2.3 The unit type `()`

The unit type is `()`, with a single value also written `()`. Functions
without a final expression per grammar §3.13 return the unit value
implicitly. The unit type appears in pattern position as `()` to match
unit-typed values and as a type expression for return types of functions
producing no meaningful value:

```
fn print_hello() -> ():
  println("hello")

fn print_hello():          // same as above; -> () may be omitted
  println("hello")
```

#### 9.2.4 The 1-tuple

The 1-tuple form requires a trailing comma to disambiguate from a
parenthesized expression:

```
let single = (42,)         // 1-tuple of type (i32,)
let grouped = (42)         // just i32 in parens — not a tuple
```

The trailing-comma convention is standard across languages with tuple
support and resolves the syntactic ambiguity cleanly.

#### 9.2.5 Generics over tuples

Generic parameters appear in tuple types using standard generic syntax;
no special mechanism is needed:

```
fn first[A, B](t: (A, B)) -> A:
  t.0

fn swap[A, B](t: (A, B)) -> (B, A):
  (t.1, t.0)
```

The tuple type `(A, B)` is a type expression like any other; `A` and `B`
are bound by the generic parameter list. Per §2.3, each unique
tuple-type instantiation produces its own specialized code.

**Variadic generics** — abstraction over tuples of arbitrary arity — are
not supported in v1. Functions generic over "any tuple" would require
either macro support or a different abstraction mechanism (e.g., a trait
with associated types for each component). May be added later if usage
patterns justify the complexity. For now,
generic-over-tuple-component-types covers the common case.

#### 9.2.6 Trait conformance for tuples

Trait conformance for tuples is supported via `fulfill` blocks per §3.3,
subject to the orphan rule from §3.7 — including the
generic-parameter-coverage rule from §3.7.2. Since tuple types are
structural and not declared in any module, the coverage check operates
on the tuple's element types:

- A `fulfill SomeTrait for (T1, T2, ...)` is permitted if `SomeTrait` is
  local *or* if at least one of the element types `Ti` is local.
- For tuples consisting entirely of foreign types (e.g., `(i32,
  string)`), the trait must be local — no element provides coverage.
- For tuples containing at least one locally-defined element type
  (e.g., `(i32, MyType)` where `MyType` is local), coverage is satisfied
  via that local element, and a foreign trait can be implemented.

```
// In user module declaring MyTrait and MyType:
fulfill MyTrait for (i32, string):          // ✓ trait is local
  ...

fulfill Display for (i32, MyType):          // ✓ MyType covers; Display is foreign
  ...

fulfill Display for (i32, string):          // ✗ both element types foreign,
                                            //   trait also foreign — orphan
  ...
```

Stdlib provides `fulfill` blocks for common tuple types implementing
common traits (`Eq`, `Ord`, `Hash`, `Clone`, `Display`, `Debug`).
Coverage extends through **tuple arity 12** — beyond arity 12, users
implement explicitly. The arity limit reflects the practical observation
that tuples larger than 12 components are rare and typically indicate
the user should be using a record (§6.1) instead.

#### 9.2.7 Tuple-to-record conversion

Tuple-to-record conversion is explicit. Tuples are structural; records
are nominal; they do not share identity, and the compiler does not
implicitly convert between them. Manual conversion uses field-by-field
construction:

```
let t = (1.0, 2.0, 3.0)
let v = Vec3(x: t.0, y: t.1, z: t.2)
```

For ergonomic repeated conversion, a `From` impl per §7 produces
method-call conversion:

```
fulfill From[(f32, f32, f32)] for Vec3:
  fn from(t: (f32, f32, f32)) -> Vec3:
    Vec3(x: t.0, y: t.1, z: t.2)

// Now:
let v: Vec3 = (1.0_f32, 2.0_f32, 3.0_f32).into::[Vec3]()
```

### 9.3 Arrays

Arrays are fixed-size, contiguous sequences of values of a single
element type. The element count is part of the type. Arrays receive
dedicated language syntax (`T[N]`) rather than being expressed through a
generic stdlib type.

#### 9.3.1 Array type syntax

```
i32[5]              // 5-element array of i32
string[10]          // 10-element array of string
f64[100]            // 100-element array of f64
```

The syntax `T[N]` is dedicated to the array type. There is no exposed
canonical `Array[T, N]` form; the underlying array representation is
internal to the compiler and not addressable by name in user code. The
syntactic shape parallels how tuples are handled — dedicated syntax with
no namespace-level type name.

**Multi-dimensional arrays** parse left-to-right: `T[N][M]` is an
M-element array of `T[N]`. To form an N-row × M-column matrix, write
`f64[M][N]` (each row is `f64[M]`; the outer array has `N` such rows).

**Zero-length arrays** `T[0]` are valid types. They are useful for edge
cases in generic code that must abstract over array sizes including
zero, and for FFI bindings to C-style flexible array members.

#### 9.3.2 Disambiguation of `T[args]` in type position

The grammar's `TypePostfixOp` is uniformly `[arg-list]`. The compiler
interprets it based on what `T` resolves to:

- If `T` is a primitive or other non-generic type, `T[args]` constructs
  the array type (e.g., `i32[5]`, `string[10]`).
- If `T` is a generic type, `T[args]` instantiates the generic with the
  given type arguments (e.g., `Vec[i32]`, `Option[string]`).

The disambiguation is by the kind of `T`, not by the kind of the
arguments. A primitive type's name is always an array-type constructor;
a generic type's name is always a generic-instantiation site. There is
no ambiguity at the parser level.

#### 9.3.3 Length type

The array length type is `isize` — signed, platform-sized. The choice
of signed reflects a real ergonomic concern: `length - 1` on an empty
array under unsigned would either wrap to `usize::MAX` (likely freezing
loops) or trap; under signed it yields `-1`, and the iteration `0..-1`
is correctly empty.

The platform-sized choice scales addressing capacity with the machine.
The theoretical halving of addressable size from `usize` to `isize` is
not a real constraint: `isize::MAX` on 64-bit platforms is
~9.2 × 10¹⁸ elements, far beyond any conceivable array.

Users needing the "must be non-negative" invariant for low-level work
(allocation sizes, FFI) can use `usize` explicitly; the language does
not block this.

#### 9.3.4 Index type

Array index types are flexible. Any integer type is accepted as an
index, implicitly widened to `isize` for the indexing operation per
§4.5's lossless-widening rules. Integer types whose value range fits
entirely in `isize`'s range widen losslessly; types whose range exceeds
`isize`'s range require explicit cast.

On 64-bit platforms (where `isize` is 64-bit), this means every integer
type up to and including `i64` widens losslessly; `u64`, `i128`, and
`u128` require explicit cast. On 32-bit platforms (where `isize` is
32-bit), the corresponding rule applies with `isize`'s narrower range.
The rule is platform-aware: the same source code is valid on every
platform, but a cast may be required on platforms with narrower `isize`
that would be unnecessary on wider platforms.

Users write indexing expressions with whichever integer type is natural
for their context — counter variables, sizes, computed offsets — and
the compiler handles the widening:

```
let i: i32 = 3
let v: i32 = arr[i]            // i32 widens to isize for indexing

let n: usize = compute()
let w = arr[n]                  // usize widens to isize for indexing

let big: u64 = some_huge()
let x = arr[big]                // ✗ compile error on 64-bit (u64 doesn't fit isize);
                                //   may also fail on 32-bit
let x = arr[big as isize]       // ✓ explicit cast
```

#### 9.3.5 Bounds checking

Bounds checking on `arr[i]` traps at runtime if `i < 0 || i >= length`,
consistent with §4.6.1's trap-on-out-of-range philosophy. The trap is
the language's signal that a logic error occurred — the program was
asked to access a position that doesn't exist.

When both the index and the length are compile-time known per §2.4,
bounds checking happens at compile time and produces a compile error on
out-of-range access:

```
let arr: i32[5] = ...
let x = arr[10]                 // ✗ compile error — 10 not in 0..5
let x = arr[3]                  // ✓ compile-time-verified safe
```

For recoverable indexing (where out-of-bounds should produce a value,
not a trap), the user calls stdlib methods like `arr.get(i)` returning
`Option[T]`, or uses the `?` variant per §4.6.4 if such an indexing
operator is provided.

#### 9.3.6 Dynamic arrays are not in the language

The dynamic-sized vector type (heap-allocated, growable) is a standard
library concern, not a language-level type. Its name and syntax (`Vec[T]`,
`Vector[T]`, or whatever stdlib chooses) is outside this specification.
Only fixed-size arrays receive dedicated language syntax. Stdlib's
dynamic collections are ordinary generic types per §2.

### 9.4 Time Types: `duration` and `instant`

The language provides two built-in time types for representing temporal
quantities:

- `duration` — a *span* of time (an interval between two moments).
- `instant` — a *point* in monotonic time (a specific moment relative to
  an implementation-defined epoch).

Both are first-class primitive types with distinct semantics, dedicated
operator rules, and (for `duration`) literal syntax. They are lowercase,
matching the convention of other primitives (`bool`, `string`, `i32`,
etc.).

#### 9.4.1 `duration`

A `duration` represents an interval of time. Internally it is i64
nanoseconds; the representation gives a range of approximately ±292
years with single-nanosecond precision. Negative durations are
permitted.

##### 9.4.1.1 Literal syntax

Numeric literals may carry one of the following built-in suffixes to
produce a `duration` value:

| Suffix | Unit         | Example      |
|--------|--------------|--------------|
| `ns`   | nanoseconds  | `500ns`      |
| `us`   | microseconds | `100us`      |
| `μs`   | microseconds | `100μs`      |
| `ms`   | milliseconds | `250ms`      |
| `s`    | seconds      | `1s`, `1.5s` |
| `min`  | minutes      | `5min`       |
| `h`    | hours        | `2h`         |
| `d`    | days         | `1d`         |

Both integer and float literals may carry these suffixes. Float literals
convert to nanoseconds with rounding-to-nearest at compile time
(`1.5s` → `1_500_000_000ns`). Integer literals scale exactly.

These suffixes are reserved by the language; `@literal_suffix` (§3.9)
may not re-register them in any scope.

##### 9.4.1.2 Operators

The following operators are defined for `duration`:

| Operation                  | Result     | Notes                          |
|----------------------------|------------|--------------------------------|
| `duration + duration`      | `duration` | sum of spans                   |
| `duration - duration`      | `duration` | difference (may be negative)   |
| `duration * Numeric`       | `duration` | scale; `Numeric * duration` ok |
| `duration / Numeric`       | `duration` | scale down                     |
| `duration / duration`      | `f64`      | ratio (canonical float result) |
| `duration % duration`      | `duration` | modulo (remainder)             |
| `-duration`                | `duration` | negation                       |
| `duration <,<=,==,!=,>=,>` | `bool`     | comparison                     |

The `Numeric` operand may be any integer or float type per §4.1. Integer
scaling is exact; float scaling rounds to nearest at the nanosecond level
before storing the result.

Division by zero follows the standard rules of §4.6:

- `duration / 0` where `0` is an integer-typed value traps per §4.6.7
  (integer division by zero).
- `duration / 0.0` where `0.0` is a float-typed value produces `±inf`
  or NaN per IEEE 754; converting that result back to the i64-nanosecond
  duration representation traps per §4.7.3 (default float-to-int cast
  traps on non-finite values).
- `duration / duration_zero` (i.e., dividing by a zero-duration value)
  traps; this is treated as the i64 zero-divisor case per §4.6.7.

Use checked variants (`/?`, `%?`) per §4.6.4 where division-by-zero
recovery is needed; they return `Option[duration]` (or `Option[f64]`
for `duration / duration`).

Operations **not defined** for `duration`:

- `duration + Numeric` / `Numeric + duration` — no implied unit.
- `duration - Numeric` / `Numeric - duration` — no implied unit.
- `duration * duration` — no semantic meaning.

Attempting any forbidden operation is a compile error.

##### 9.4.1.3 Overflow

Default arithmetic operators trap on overflow per §4.6.1: a `duration`
result that does not fit i64 nanoseconds aborts the process. Checked
variants (`+?`, `-?`, `*?`, `/?`, `%?`) per §4.6.4 return
`Option[duration]` and are recommended where saturation or failure
recovery is needed.

##### 9.4.1.4 Construction and conversion (stdlib)

Construction from raw integer/float values and conversion to integer/float
counts are stdlib concerns, not language built-ins. Stdlib is expected to
provide:

- `duration::from_nanos(n)`, `from_micros(n)`, `from_millis(n)`,
  `from_secs(n)`, `from_minutes(n)`, `from_hours(n)`, `from_days(n)`.
- `d.as_nanos() -> i64` — lossless (nanoseconds is the internal repr).
- `d.as_micros() -> i64`, `d.as_millis() -> i64`, `d.as_secs() -> i64` —
  truncate sub-unit components.
- `d.as_secs_f64() -> f64`, `d.as_millis_f64() -> f64`, etc. — float
  variants for ratio-style queries (precision-bound by f64).

#### 9.4.2 `instant`

An `instant` represents a *monotonic* point in time — a value from a
clock that never goes backward, measured against an implementation-
defined epoch (typically program start or system boot).

Instants are opaque: there is no literal syntax for them, no direct
construction from raw nanoseconds, and no operations that expose the
underlying value as an absolute count. Their purpose is type-level
distinction from `duration` and from arbitrary integers.

Internally, an `instant` is represented as i64 nanoseconds since the
implementation-defined epoch, paralleling `duration`'s representation,
but the value is exposed only via comparisons and difference operations.

`instant` represents monotonic time. Wall-clock time (calendar dates,
timezones, DST) is a separate concern, deferred to stdlib. The language
core defines only monotonic instants.

##### 9.4.2.1 Operators

The following operators are defined for `instant`:

| Operation                 | Result     | Notes                           |
|---------------------------|------------|---------------------------------|
| `instant - instant`       | `duration` | elapsed time between two points |
| `instant + duration`      | `instant`  | future point                    |
| `instant - duration`      | `instant`  | past point                      |
| `instant <,<=,==,!=,>=,>` | `bool`     | comparison                      |

Operations **not defined** for `instant`:

- `instant + instant` — no semantic meaning.
- `instant * Numeric` / `instant / Numeric` — scaling a point in time
  has no meaning.
- Any direct arithmetic between `instant` and `Numeric` — no implied
  unit.

##### 9.4.2.2 Construction (stdlib)

Constructing an `instant` requires the host's clock; the language
core does not provide direct construction. Stdlib is expected to
provide:

- `instant::now() -> instant` — returns the current monotonic time.
- No conversion to absolute values (would imply wall-clock semantics
  the language does not commit to).

Comparison and difference are the canonical operations on `instant`;
no other introspection is provided.

##### 9.4.2.3 Limitation: no cross-run serialization

Because the `instant` epoch is implementation-defined and tied to a
single kernel run (typically program start), instants cannot be
reliably serialized to disk and restored across runs — a saved
`instant` from one run has no meaningful interpretation in a later
run started from a different epoch. Programs that need persistent
absolute time must use stdlib serialization that converts instants
to and from a stable absolute representation (e.g., Unix nanoseconds
since 1970-01-01 UTC). The language core does not provide this; it
is a stdlib concern.

#### 9.4.3 Reactive cell compatibility

Both `duration` and `instant` are i64-sized values and satisfy the
direct in-cell storage criteria of §13.12.4. They may appear directly
as the type of `signal`, `attr`, `recurrent`, and `derived`
declarations.

Wrapping in `Result[duration, E]`, `Option[duration]`,
`Result[instant, E]`, or `Option[instant]` is governed by §13.12.4's
general storage rules: if the total bit width (discriminant + payload)
fits the platform atomic word, direct storage applies; otherwise the
cell uses handle-based pool storage. On platforms supporting wide
atomics, `Option[duration]` (≈9 bytes) fits a 128-bit-coupled cell;
on platforms without wide atomics, it falls back to handle-based
storage. The compiler chooses the strategy; the source-level type is
permitted in all cases.

For minimum-overhead reactive cells, prefer bare `duration` / `instant`
when an absent/errored sentinel can be encoded in the value's range
(e.g., `i64::MIN`) rather than via `Option`/`Result`.

---

## 10. Visibility and Modules

The language uses a three-level visibility model — `public`, `shared`,
and `private` — and a folder-as-module structure for organizing code
within and across packages. This section is the authoritative
specification for both. Earlier sections cross-reference here for
declaration-specific behavior.

### 10.1 The Three Levels

Visibility is three-level. Each level denotes a distinct scope:

| Level     | Scope                                                                    | Default? |
|-----------|--------------------------------------------------------------------------|----------|
| `public`  | Across package boundaries — exported to dependent packages               | no       |
| `shared`  | Within the same package (the module tree rooted at the package root)     | **yes**  |
| `private` | Within the declaring module only (the folder containing the declaration) | no       |

`shared` is the default; no keyword is required. `public` and `private`
are explicit keywords.

The three levels are linearly ordered by permissiveness:
`private < shared < public`. A declaration's visibility level determines
the maximum reach of any reference to it; references from outside that
reach produce compile errors at the reference site.

The unit of `private` is the *module* (the folder), not the file
within it. Files inside the same folder are organizational; they
share scope and see each other's declarations regardless of
visibility level (§10.2.1).

### 10.2 Packages and Modules

A *package* is the unit of distribution — a project root or a named
dependency. Each package has a single *package root*: the top-level
folder of the package's source tree. The package root is itself the
*root module*, addressed in absolute paths via the `root` keyword.
Each subfolder of the package root is a distinct module addressable
by its folder name (e.g., `<package_root>/audio/` is the module
`root::audio`).

A *module* is a folder that contains one or more `.duc` source
files directly inside it. The folder's path within the package
determines the module's path; files inside the folder are
organizational, not separately addressed. There is no module marker
file, no module declaration inside files, no manifest. **A folder
is a module iff it contains `.duc` files.**

A folder *without* any `.duc` files is not a module — it is a pure
path-segment folder. Such folders are filesystem organization only;
they cannot be the target of a `use` statement or qualified path
reference, and they have no declarations of their own.

Path-segment folders **do not** prevent their subfolders from being
modules. A subfolder of a path-segment folder is a module if it
itself contains `.duc` files; its module path is constructed by
traversing the path segments and the parent's module path normally.

```
root/
├── main.duc                  // root module (has .duc directly)
├── audio/                    // path segment only (no direct .duc)
│   ├── synth/
│   │   └── synth.duc         // root::audio::synth module
│   └── effects/              // path segment only (no direct .duc)
│       └── reverb/
│           └── reverb.duc    // root::audio::effects::reverb module
```

Use sites resolve through path segments unchanged:

```
use root::audio::effects::reverb::Reverb    // ✓ resolves through audio/effects/
use root::audio::*                           // ✗ audio/ is not a module
use root::audio::effects                     // ✗ audio/effects/ is not a module
```

A subfolder is *not* a "child module" or "submodule of its parent"
in any special sense — each module folder is an independent module
addressable by its own path. References between them are ordinary
cross-module references.

Mixing code and non-code in the same folder is permitted by the
language but is the developer's organizational responsibility; the
language imposes no convention beyond the rule above.

#### 10.2.1 Files within a module

Files inside the same folder share scope: each file sees all
declarations from sibling files automatically, without any `use`
statement or path qualification. The unit of identity in the type
system is the module (the folder); files inside are purely
organizational means of splitting source across multiple physical
files.

```
<package_root>/
├── main.duc           // part of root module; sees signals.duc declarations directly
├── signals.duc        // part of root module
└── audio/
    ├── oscillator.duc // part of root::audio; sees filter.duc directly
    └── filter.duc     // part of root::audio; sees oscillator.duc directly
```

In this layout: `main.duc` and `signals.duc` are both in the root
module and reference each other's declarations with no import.
`oscillator.duc` and `filter.duc` are both in `root::audio` and
likewise reference each other directly. But `main.duc` referencing
something from `audio/oscillator.duc` requires either a `use`
statement or a qualified path (§10.4) — those are different modules.

No file declares which module it belongs to; the folder location is
the source of truth.

#### 10.2.2 Visibility reach

The three visibility levels translate to declaration reach as follows:

- A `private` declaration is reachable from any file inside its
  *module* (the folder it is declared in). Sibling files in the same
  folder see it. Files in any other module — any other folder of the
  same package, or any external package — cannot reference it.
- A `shared` declaration is reachable from any file within the same
  package, including the declaring module's siblings and files in any
  other module of the same package. Cross-module access within the
  package requires either a `use` statement (§10.4) or a path-qualified
  reference.
- A `public` declaration is reachable from any file within the same
  package (as for `shared`), plus any file in any package that depends
  on the source package. Cross-package references use the importing
  package's external dependency path base; see §10.2.3.

Within a single module, all three levels behave identically — every
declaration is visible to every sibling file regardless of its
visibility specifier. The visibility level only matters for
references *outside* the declaring module.

Cross-module access always requires explicit reference, either via
`use` (§10.4) or via path qualification.

#### 10.2.3 Path bases

The grammar's `PathBase` (per grammar §3.4) provides the following entry
points for absolute paths:

- `root` — the current package's root module.
- A bare name matching an external dependency declared in the package's
  manifest — that dependency's root module.
- `std` — the standard library's root module. Built-in path base,
  implicitly available to every package without manifest declaration.
  Stdlib types and functions are accessed through this base (e.g.,
  `std::option::unwrap`, `std::vec::Vec`).

For example, `root::audio::Synthesizer` resolves an absolute path
through the current package; `tone_lib::Oscillator` resolves into the
`tone_lib` dependency's public surface; `std::vec::Vec` resolves into
the standard library.

All `use` statements use absolute paths starting from one of these
bases. There is no relative-path or "current module" reference;
references between modules always go through `root` or an external
dependency name.

### 10.3 Visibility Specifiers on Declarations

Every position in the grammar that admits a visibility specifier
accepts one of: `public`, `shared`, `private`, or *absence* (which
denotes `shared` by default). The grammar's older `pub` keyword is
replaced throughout by this three-level model; the propagation covers
all visibility-bearing productions (grammar §3.4 through §3.11).

```
public fn render_frame(...): ...           // exported across packages
fn compute_delta(...): ...                 // shared (default)
private fn internal_helper(...): ...       // module-local

public type Synthesizer:                   // type public
  ...

private const SECRET_KEY: u64 = 0xDEADBEEF // module-local constant
```

Specific visibility rules for each declaration kind are specified in the
declaration's own section and summarized below:

- **Records** (§6.1): type visibility (§6.1.7), independent field
  visibility (§6.1.6), independent constructor visibility (§6.1.7).
- **Enums** (§6.2): type visibility applies uniformly to all variants
  (§6.2.6); no per-variant visibility.
- **Newtypes** (§6.3): type visibility (§6.3.1), independent
  constructor visibility (§6.3.4).
- **Alias types** (§4.2, §10.4.2): visibility specifier on the
  `alias type` declaration.
- **Traits** (§3.1): type visibility. Visibility of methods within a
  trait declaration is uniform with the trait's visibility — no
  per-method visibility.
- **Free functions**: visibility specifier on the `fn` declaration.
- **Operators** (§13.17): visibility specifier on the `operator`
  declaration; same rules as functions.
- **Constants** (§2.4.1.1): visibility specifier on the `const`
  declaration.
- **Reactive declarations** (§13.2.1, §13.2.3, §13.2.4): module-level
  `signal`, `derived`, and `recurrent` accept visibility specifiers
  on the same line as the declaration.
- **Node and connection types** (§13.3, §13.6): visibility specifier
  on the type declaration.
- **Instantiations**: any top-level placement (`Foo bar:`,
  `signal x = ...`, `let y = ...`) accepts a visibility specifier.
  An instantiation is conceptually `let-binds-an-instance`;
  visibility controls cross-module reachability of the instance
  name.
- **Fulfill blocks** (§10.8): no separate visibility specifier —
  reachability derived from trait and type visibility jointly.

Visibility specifiers attach to any *named declaration*. The unifying
rule: if a name is introduced into a scope, the declaration may carry
a visibility specifier governing that name's cross-scope reach.

### 10.4 `use` Statements

A `use` statement imports a name from a *different module* into the
current file's scope, allowing the file to refer to that name
unqualified rather than via its full path. The grammar of `use` is
specified in grammar §3.3.

```
use root::audio::Synthesizer

let s = Synthesizer(...)              // unqualified — would be
                                      // root::audio::Synthesizer(...) otherwise
```

`use` statements are required only for *cross-module* references.
Files within the same module (the same folder) see each other's
declarations automatically; no `use` is needed for siblings (§10.2.1).

A `use` statement is **per-file**: it affects only the file in which
it appears. Sibling files in the same module each declare their own
`use` statements for the external names they need. There is no
mechanism to share imports across sibling files.

All `use` paths are absolute, starting from a path base
(`root` or an external dependency name; see §10.2.3). There is no
relative-path form.

`use` has **no visibility modifier**. It is a usage-side construct: it
controls how the current file refers to other names, not how other files
refer to the current file. A name brought into scope via `use` does not
become a declaration in the current file; it remains the original
declaration in the original module, just with a shorter local reference.

The visibility of the imported declaration governs whether the `use` is
permitted at all. Importing a `private` declaration from a different
module is a compile error (the source isn't visible from outside its
module). Importing a `shared` declaration from within the same package
works; importing it from another package does not. Importing a
`public` declaration works from any package that depends on the
source's package.

A `use` statement targets the module path of the declaration's
*module* (folder), not the file within it. From the importer's
perspective, the file in which the declaration physically lives is
irrelevant — only its module's path matters.

#### 10.4.1 Selective and glob imports

Per §6.2.3, selection lists on `use` paths use parentheses; a glob
imports every visible name from the source:

```
use root::ops::(add, sub, mul)        // specific names
use root::variants::*                 // glob: every visible name
```

Glob imports are subject to the import-time conflict rules per §6.2.3:
two glob imports that bring colliding names into the same scope produce
a compile error at the `use` site that introduces the second collision.

#### 10.4.2 Re-exporting a name

To make a declaration accessible from another module under a different
path, write an explicit re-declaration rather than a re-exporting
`use`. Common forms:

```
// In root::facade.duc:
public alias type Synthesizer = root::audio::internal::Synthesizer
                                       // alias type form (§4.2)

public fn build_default() -> Synthesizer:
  root::audio::internal::build_default_with_params(...)
                                       // wrapper function
```

These are ordinary declarations with their own visibility specifiers,
distinct from `use` imports. The language's `use` machinery is solely
about bringing names into the current file's scope; cross-module
exposure of names is the job of declarations.

Visibility specifiers (`public`, `shared`, `private`) are permitted on
`alias type` declarations and follow the same rules as other declarations
per §10.3 (which enumerates `alias type` among the visibility-bearing
forms).

#### 10.4.3 `use` is file-scope only

A `use` statement may appear only at file scope (alongside other
top-level declarations). Function-scope `use` (a `use` statement
inside a function body, block, or other inner scope) is a compile
error. Local short names within a function body are achieved by
binding the desired value to a `let` or `mut` (e.g.,
`let synth = root::audio::Synthesizer`), not by importing the name.

This restriction keeps the import surface of a file visible at the
top of the file, which aids tooling, navigation, and reasoning
about dependencies. It also avoids the complexity of nested-scope
import shadowing.

#### 10.4.4 Circular module references are forbidden

The cross-module `use`-and-reference graph must be acyclic. If any
cycle exists — a chain of modules where each references the next
and the chain eventually returns to its starting module — the
cycle is rejected at compile time. Binary cycles (A→B→A) and
longer cycles (A→B→C→A, etc.) are equally forbidden. The error
identifies the cycle's members.

**Within-module sibling cycles — distinguish two kinds:**

- **Type-reference cycles between sibling files are permitted.**
  Files inside the same module share scope and are compiled as a
  single unit, so mutually-referencing type declarations (e.g.,
  one file's `node` declares an `out:` connection type defined in
  a sibling file, and the sibling's `connection` declares `from:`
  the first file's node) are resolved in one pass. This is the
  normal case for any non-trivial module split across files.
- **Initializer-reference cycles between sibling files are
  forbidden.** If file A's top-level initializer (a const value,
  signal initial value, attr default at module scope) depends on
  a name from file B, and B's initializer depends on a name from
  A, the cycle is rejected (§10.4.5). Compile-time-resolvable
  type references and runtime-evaluated initializer references
  are evaluated under different rules.

This split rule eliminates ambiguous initialization order while
preserving the convenience of multi-file modules for type
declarations.

Programs that need shared state between mutually-referencing
modules must extract the shared declarations into a third module
that both depend on, breaking the cycle topologically. This applies
to both cross-module cycles and within-module initializer cycles.

Note: this rule applies to the *static reference graph* (use
statements, path-qualified references, type references between
sibling files, initializer-time references). It is distinct from
reactive-graph cycles (§13.11), which operate at the runtime
dependency level and have their own rules.

#### 10.4.5 Cross-module initialization order

Top-level declarations with initializers — `const`, `signal`, and
the placement-time-evaluated portions of node/connection bodies —
are initialized in **topological order** of the cross-module
reference graph. If module A's initializers reference items from
module B, B is initialized before A. Because circular module
references are forbidden (§10.4.4), this ordering is well-defined.

Within a single module (across all sibling files in the same
folder), the within-module initialization order is:

1. **Topological across files** based on cross-file initializer
   references. If file A's initializer references a name from file
   B, B's initializer runs before A's. Cycles in initializer
   references between sibling files are a compile error (note:
   *type* references between sibling files may form cycles — see
   §10.4.4 — but *initializer* references must not).
2. **Source declaration order within each file.** Among
   declarations in the same file, the textually earlier one
   initializes first.

Per-section rules (§13.2.6 for reactive declarations; analogous
rules for plain consts and signals) refine these for specific
declaration kinds.

The compiler computes the topological order at compile time;
runtime initialization follows this fixed order. A program never
observes initialization in any order other than the topologically-
determined one.

### 10.5 Type Visibility and Constructor Visibility

Records (§6.1.7) and newtypes (§6.3.4) carry an independent constructor
visibility specifier alongside the type visibility. The syntax uses a
parenthesized modifier on the type visibility keyword:

```
public type Email:                        // newtype; type public, constructor public (default)
  wraps string

public(shared) type Email:                // newtype; type public, constructor shared
  wraps string

public(private) type Email:               // newtype; type public, constructor private
  wraps string                            //   — the smart-constructor pattern

shared(private) type SecretConfig:        // record; type shared, constructor private
  api_key: string
```

The outer keyword is type visibility; the parenthesized inner keyword is
constructor visibility. When the inner specifier is omitted, constructor
visibility defaults to match the type's visibility.

**Inner ≤ outer.** The inner specifier may never be *more* permissive
than the outer. `private(public)` is a compile error — the inner
specifier claims wider reach than the enclosing type permits. The
constructor's effective reach is capped at the type's reach; declaring
a broader constructor visibility produces no additional access and is
rejected to surface the inconsistency at the declaration site.

#### 10.5.1 The smart-constructor pattern

The `public(private)` and `shared(private)` configurations are the
canonical smart-constructor pattern: the type's name is visible across
its visibility scope (so callers can use it in signatures, annotations,
and field types), but construction `TypeName(...)` is unreachable from
outside the constructor's scope.

This is the language's mechanism for enforcing invariants at
construction time. Any path that produces a value of the type must pass
through the constructor's visibility scope, where validating logic — a
`From` impl, a `TryFrom` impl, a factory function — can be defined.
Callers receive values of the type that have passed the invariants;
they cannot manufacture invalid values directly.

### 10.6 Enum Visibility

Enum visibility applies uniformly to the enum type and all its variants
(§6.2.6). There is no per-variant visibility specifier.

```
public enum Color:                        // all variants public
  Red
  Green
  Blue

private enum InternalState:               // type and all variants module-local
  Pending
  Running
  Done
```

If a user needs some variants visible and others hidden, they split the
enum into multiple enums (each with its own visibility) and provide
conversion functions between them. The motivation: per-variant
visibility is rare in practice; supporting it would complicate the
grammar and module-resolution rules for narrow benefit.

### 10.7 Field Visibility

Records carry independent visibility per field (§6.1.6). Each field
declares its own visibility:

```
public type Account:
  public id: i64                  // readable anywhere the type is visible
  email: string                   // shared (default)
  private password_hash: string   // readable only within this module
```

A field's visibility never exceeds the enclosing type's visibility —
declaring a `public` field on a `private` type is a compile error,
because no caller outside the type's visibility scope could observe the
field.

Access from outside a field's visibility scope is a compile error at
the access site.

### 10.8 Trait `fulfill` Block Visibility

`fulfill` blocks (§3.3) have *no separate visibility specifier*. The
implementation's effective visibility is:

```
impl_visibility = min(trait_visibility, type_visibility)
```

where the visibility levels are ordered `private < shared < public`.
An implementation is callable wherever both the trait and the type are
visible — the intersection of their reachability.

Concrete cases:

| Trait visibility | Type visibility | Impl visibility                        |
|------------------|-----------------|----------------------------------------|
| `public`         | `public`        | `public` (anywhere both are visible)   |
| `public`         | `shared`        | `shared` (package-internal)            |
| `shared`         | `public`        | `shared` (package-internal)            |
| `private`        | `public`        | `private` (only in the trait's module) |
| `private`        | `private`       | only if both declared in same module   |

The intersection rule reflects the practical observation: if a caller
can't name both the trait and the type, the implementation is
unreachable from that caller's site regardless of any separate
visibility specifier on the `fulfill` block.

The motivation for *not* having a separate visibility specifier: a
separate specifier could create the case where the trait and type are
both visible but the implementation is not, leading to confusing
"method not found" errors when the implementation clearly should exist.
Coherence per §3.7 guarantees at most one implementation exists per
(trait, type) pair, so there is no ambiguity in which implementation is
the visible one — only whether it is reachable.

### 10.9 Visibility and the Orphan Rule

The orphan rule (§3.7) operates on *package-of-declaration*, not on
visibility. A `fulfill` block satisfies the orphan rule if the trait
or the type is declared in the current package (the same package
the `fulfill` block resides in) — regardless of either's visibility
level. Visibility controls *who can see and use* an implementation;
the orphan rule controls *where it can be declared*.

A `private` trait or type still counts as "in the current package"
for orphan-rule purposes. The combination — a `fulfill` block for a
private trait and a foreign type, with the implementation accessible
only inside the declaring module — is rare but valid.

### 10.10 Visibility and Dispatch

Visibility interacts with the uniform call syntax (§3.4) through name
resolution. A method call `x.f()` resolves `f` against names visible in
the current scope; visibility determines which names are reachable from
the call site:

- A `private` function is reachable only from within its declaring
  module.
- A `shared` function is reachable from any file within the same package
  via a `use` statement bringing it into scope, or via path
  qualification.
- A `public` function is reachable as for `shared`, plus from any file
  in any package depending on the source package.

In all cross-module cases — same-package or cross-package — the
reference is explicit: either the name is brought into scope via
`use`, or the call uses a path-qualified form like
`root::module::function_name(args)`. Within a module, sibling files
see each other's declarations directly (§10.2.1); no explicit
reference is needed.

The resolution algorithm per §3.4.1 searches in-module declarations
and imported names in the current file; visibility filters which
names can be successfully brought into scope or referenced via path.
Trait-method calls follow the same rule, with the additional reach
constraint from §10.8 — the implementation's effective visibility is
the minimum of the trait's and type's visibility.

---

## 11. Local Mutability and Ownership

This section specifies the language's local mutability model. Mutation is
permitted only inside function bodies, scoped to bindings declared with
`mut`. Ownership of values is tracked through move semantics: every value
has exactly one owner at any moment. Read-only access to non-`Copy` values
without ownership transfer is provided through call-scoped borrows declared
in function signatures.

This section supersedes the absolute-immutability language in §1.3. The
broader principle stands — immutability is the default and external state
remains immutable — but local mutation is permitted inside function bodies
as a controlled escape hatch for performance.

### 11.1 Design Principles

Mutation in Ductus is an escape hatch, not the primary expression style.
The default remains immutability and pure functions; `mut` exists because
some computations (DSP buffer processing, in-place transformations,
algorithm internals) cannot be expressed efficiently in a pure-functional
style. The model is designed to *isolate* mutation rather than eliminate
it.

Three load-bearing rules constrain where and how mutation can occur:

**Nothing outside a function body is mutable.** Module-level bindings,
record fields as a property of the type, function parameters, enum
variants — none of these can be declared `mut`. The `mut` keyword is
legal only on bindings introduced inside a function body.

**Single ownership.** At every moment, every value has exactly one owner.
Passing a non-`Copy` value into a function transfers ownership. Returning
it transfers ownership back to (a new binding in) the caller. Assigning a
non-`Copy` value to a new binding transfers ownership. The compiler tracks
ownership at every binding site; using a value after ownership has been
transferred is a compile error.

*Exception — reference-typed reactive bindings.* `Signal[T]` parameters
(§13.2.8) and reactive composite bindings (§13.2.9.6) name reactive
cells (specified by §13, §14) rather than stack-owned values;
multiple live aliases to the same cell may coexist without violating
single ownership. For reactive composites, materialization at the
boundaries of §13.2.9.7 produces a concrete instance subject to
standard single-ownership rules from that point on.

**Single writer.** A `mut` binding is the only path through which its
underlying value may be mutated. While a borrow of the value is active,
even the owner cannot mutate it. The compiler enforces this without any
runtime check.

The result is a model where mutation is locally efficient (no copying for
in-place updates) but globally invisible (no caller can observe a callee's
mutations except through the callee's declared return value). This
combination preserves the language's pure-functional surface (functions
remain referentially transparent observably) while permitting imperative
implementation underneath.

### 11.2 Binding Forms: `let` and `mut`

The language has two binding forms for runtime values:

```
let x = expr        // immutable binding
mut x = expr        // mutable binding (function bodies only)
```

`let` is the general-purpose binding form, identical to the form specified
in §2.1.2 and §2.4.1.1. The binding is immutable: the binding name cannot
be reassigned, the bound value cannot be mutated through this binding, and
field/element assignment through this binding is a compile error.

`mut` is the local-mutability binding form. The binding name can be
reassigned, the bound value can be mutated in place (through indexed
assignment, field assignment, or whole-value reassignment), and the
binding lives only within the function body where it is declared.

`mut` is **forbidden at module top level**, **forbidden inside type, trait,
node, and connection bodies**, and **forbidden on function parameters**.
Only function bodies (and nested block scopes within function bodies) may
contain `mut` declarations. The grammar and the type checker both enforce
this; a `mut` declaration outside a function body is a compile error at
the declaration site.

The `const` binding form (§2.4.1.1) remains valid as the strictly
compile-time-only form. `const` and `mut` are mutually exclusive — `const`
asserts compile-time-only and immutable; `mut` is necessarily runtime and
mutable.

#### 11.2.1 Shadowing

Either form may shadow a previously declared binding in the same scope:

```
fn process(input: Vec[i32]) -> i32:
  let input = preprocess(input)       // shadows the parameter
  let input = filter(input)           // shadows again
  sum(input)
```

Shadowing creates a new binding with the same name; the prior binding is
no longer accessible by that name from the shadow point forward. Under
move semantics this is the idiomatic pattern for "thread a value through
a pipeline" — each step rebinds the same name to the new owned value.

A `let` may shadow a `mut` and vice versa. The new binding's mutability is
governed solely by its own declaration form, not by what it shadowed.

### 11.3 Ownership and Move Semantics

Every value has exactly one owner. The owner is the binding (or temporary
expression slot) that currently holds the value. Ownership transfers in
three situations:

- **Assignment.** `let y = x` or `mut y = x` transfers ownership of `x`'s
  value to `y`. After this point, `x` is no longer accessible by that name.
- **Function argument passing.** `f(x)` transfers ownership of `x` into the
  function's parameter binding for the duration of the call. After the
  call, `x` is consumed; the caller's binding is no longer usable.
- **Function return.** `return e` transfers ownership of `e`'s value out
  of the function and into whatever binding (or expression) receives the
  return value at the call site.

Ownership transfer is what "move" means. The compiler tracks ownership
statically; using a binding after its value has been moved is a compile
error reported at the use site, with the location of the move identified.

```
let v = make_buffer()       // v owns the buffer
let w = v                   // ownership moved from v to w; v no longer usable
print(v)                    // ✗ compile error: v consumed at line 2
```

#### 11.3.1 Reading versus consuming

The owner of a value may *read* through it without consuming it. Reading
includes:

- Field access: `r.field`
- Indexed access: `arr[i]`
- Pattern matching with read-only patterns
- Built-in operator inspection (`is`, `<`, etc.)

Reading does not transfer ownership. The owner retains the value after
the read.

```
let r = make_record()
print(r.first_name)         // reads r.first_name; r still owned
print(r.last_name)          // reads again; r still owned
consume(r)                  // consumes r; ownership moved
print(r.age)                // ✗ compile error: r consumed at line 4
```

Consuming includes:

- Function argument passing (by-value parameters): `f(r)`
- Return statements: `return r`
- Assignment to a new binding: `let x = r`
- Storing in a record field, tuple component, or enum payload

These operations transfer ownership.

#### 11.3.2 Reassignment of `mut` bindings

A `mut` binding may be reassigned. Reassignment consumes the new value
and drops the old value:

```
mut buf = make_buffer()
buf = make_other_buffer()    // old buffer dropped, new one bound
```

Reassignment is *not* shadowing — it modifies the existing binding rather
than introducing a new one. The binding remains the same; only the value
it holds changes.

#### 11.3.3 Dropping

When a binding goes out of scope, its value is dropped. For `Copy` types,
dropping is a no-op. For non-`Copy` types whose constituent resources
require cleanup (heap allocations, file handles via stdlib, etc.), the
type's drop behavior is invoked.

Drop semantics for user-defined types are specified through the trait
system; the precise mechanism is specified in §14.8.

### 11.4 The `Copy` Trait

`Copy` is a language-defined marker trait (§3.7.4). A type's values may
be duplicated by the language at every use site (assignment, argument
passing, return) without transferring ownership. The original binding
remains usable.

```
trait Copy
```

No methods. No associated types. The trait's only purpose is to flag a
type as having implicit-duplication semantics.

Non-primitive types opt into `Copy` either via `@derive(Copy)` (idiomatic
shorthand) or via explicit `satisfies Copy` in the type's body. Both
forms are valid and have identical semantics: the compiler verifies the
structural requirement that every field's type is itself `Copy`, then
enables implicit-duplication semantics for values of the type. The
`@derive(Copy)` form is preferred for parallel with other derivable
traits.

#### 11.4.1 Auto-implementations

The following types automatically implement `Copy`:

- All primitive numeric types (`i8`–`i128`, `u8`–`u128`, `isize`, `usize`,
  `f32`, `f64`).
- `bool`, `char`.
- `string` (see §11.6).
- `duration`, `instant` (see §9.4; both are i64-sized scalars).
- Tuples whose components are all `Copy`.
- `Range[T]` when `T: Copy` (language-privileged implementation
  per §3.7.3).

#### 11.4.2 Opt-in via `@derive(Copy)`

A record may opt into `Copy` semantics by including `@derive(Copy)` in its
declaration:

```
@derive(Copy)
type Color:
  r: u8
  g: u8
  b: u8

@derive(Copy, Eq, Hash)
type Vec3:
  x: f32
  y: f32
  z: f32
```

`@derive(Copy)` requires every field's type to itself be `Copy`. If any
field is non-`Copy` (e.g., contains an array or a non-`Copy` user type),
the derivation fails with a compile error identifying the offending field.

A newtype may opt into `Copy` similarly; the wrapped type must be `Copy`.

#### 11.4.3 Semantics of `Copy` use sites

For `Copy` types, every operation that would normally transfer ownership
instead produces an independent value:

```
@derive(Copy)
type Point:
  x: f32
  y: f32

let p = Point(x: 1.0, y: 2.0)
let q = p                         // q is an independent Point; p still usable
let total_x = p.x + q.x           // both readable
plot(p)                            // does not consume p
plot(q)                            // does not consume q
print(p.x)                         // ✓ p still owned
```

The duplication is conceptually a value-by-value copy. The runtime cost
is whatever the type's representation makes it (a register copy for small
types, a memcpy for larger ones). The language guarantees the user-visible
behavior; the compiler picks the implementation.

#### 11.4.4 `Copy` and `mut`

A `mut` binding to a `Copy` type behaves the same as for non-`Copy` types
with respect to mutation — the binding's value can be reassigned and (for
record/tuple `Copy` types) fields can be assigned. Other (immutable)
bindings to copies of the same value are unaffected by mutations made
through one `mut` binding, because they hold independent copies.

```
@derive(Copy)
type Counter:
  value: i32

let original = Counter(value: 0)
mut working = original              // independent copy
working.value = 5                   // mutates working's copy
print(original.value)               // 0; original unchanged
print(working.value)                // 5
```

This is the standard interpretation of value-type mutation in
copy-semantic languages.

### 11.5 The `Clone` Trait

`Clone` is the trait for explicit deep duplication:

```
trait Clone:
  fn clone(value: &Self) -> Self
```

The method takes a borrow (§11.9) of the source value and returns an
independent owned copy.

Where `Copy` produces implicit duplications with no syntactic marker,
`Clone` requires an explicit `.clone()` call at every duplication site.
The visible call signals that an allocation (or analogous resource
duplication) may be occurring.

#### 11.5.1 Auto-derivation

`Clone` is one of the derivable traits per §3.8:

```
@derive(Clone)
type Buffer:
  data: f32[1024]
  sample_rate: i32
```

For records, derived `Clone` clones each field. Every field's type must
itself be `Clone`. For enums, derived `Clone` clones the payload of the
active variant.

#### 11.5.2 Relationship to `Copy`

`Copy` types are trivially `Clone` — the implicit duplication mechanism
provides a `.clone()` method that returns the same result as direct use.
The compiler auto-derives `Clone` for every `Copy` type.

The converse is not true: most `Clone` types are not `Copy`. Heap-allocated
structures (`Vec`, `HashMap`), arrays, and records containing them are
`Clone` (when their fields support it) but not `Copy`.

#### 11.5.3 Usage

`Clone` is invoked when the user needs two owned copies of a non-`Copy`
value:

```
let buf = make_buffer()
let backup = buf.clone()          // explicit deep copy
process(buf)                       // buf consumed
restore(backup)                    // backup still owned
```

The clone allocates as the type requires. Users who write `.clone()` are
making the cost visible.

### 11.6 Strings and the `Copy` Implementation

`string` is a `Copy` type despite being heap-allocated. Per §9.1, strings
are UTF-8 encoded sequences and are immutable. The implementation realizes
`Copy` semantics through refcounted shared backing: assigning, passing, or
returning a `string` increments a refcount on the underlying byte storage
without copying bytes; dropping a `string` decrements the refcount and
deallocates when it reaches zero.

This is observable to the user only through:

- Performance: `let t = s` is constant-time regardless of string length.
- Mutation: irrelevant, since `string` is immutable; the refcount-shared
  backing is never visible because nothing can write through it.

The user-visible model is simply: `string` is `Copy`. Passing strings to
functions does not consume them; using a string in multiple places does
not require `.clone()`.

#### 11.6.1 Why arrays are different

Arrays (§9.3) are not `Copy`, regardless of element type or compile-time
size. Implicit duplication of arrays would either require deep copy
(silent allocation per `let t = arr`, defeating the language's
performance goals) or refcounted shared backing (which is unsafe for
arrays because `mut` bindings to arrays support in-place mutation —
sharing the backing would let one binding see another's writes,
violating single-writer).

Strings escape this trade-off by being immutable. There is no `mut`
operation on a string that mutates its bytes; every "modification" returns
a new string. Refcount-shared immutable backing is therefore safe. For
arrays, no such immutability exists, so shared backing is unsafe.

The user-visible rule: strings are `Copy`, arrays are not. The
implementation difference is the immutability constraint.

### 11.7 Function Parameters

A function parameter declares either *by-value* ownership transfer or
*by-borrow* read-only access. The declaration uses the parameter's type
position:

```
fn consume_buffer(b: Vec[f32]) -> Vec[f32]: ...      // by-value: takes ownership
fn inspect_buffer(b: &Vec[f32]) -> isize: ...        // by-borrow: read-only access
```

The `&` prefix on the parameter's type denotes a borrow. Without the
prefix, the parameter is by-value.

#### 11.7.1 By-value parameters

A by-value parameter receives ownership of its argument. For `Copy`
argument types, this is equivalent to making an independent copy and
binding it to the parameter — the caller's binding remains usable. For
non-`Copy` types, the caller's binding is consumed; using it after the
call is a compile error.

The function body owns the parameter for the duration of the call. It
may read the parameter, pass it to other functions (transferring ownership
further), return it, or let it drop at function exit.

A by-value parameter is itself an immutable binding inside the function
body — like a `let` binding. To mutate it, the body must rebind it to a
local `mut` binding (§11.7.3).

#### 11.7.2 No `mut` on parameters

A function parameter may not be declared `mut`:

```
fn process(mut buf: Vec[f32]) -> Vec[f32]: ...    // ✗ compile error
```

The forbid is intentional. A function's signature is its contract with
callers; that contract specifies type and ownership behavior. Mutation is
the function's internal implementation choice — invisible to callers
because the caller's binding has already been consumed (for non-`Copy`
parameters) or never affected (for `Copy` parameters). Exposing mutation
in the signature creates ambiguity about whether the function is pure
without changing what the function can actually do.

A function that intends to mutate its parameter rebinds it to a local
`mut`:

```
fn apply_gain(buf: Vec[f32], gain: f32) -> Vec[f32]:
  mut local = buf
  // mutate local in place
  local
```

The rebind is one line per function and surfaces the moment where the
received parameter transitions to mutable working state.

#### 11.7.3 Local rebinding to `mut`

The rebind pattern `mut local = parameter` moves the parameter's value
into a fresh `mut` binding. After the rebind, `local` is mutable and
`parameter` (now consumed) is no longer accessible.

For `Copy` parameter types, the rebind produces an independent mutable
copy; mutations to `local` are invisible to the (also-still-usable)
parameter binding. For non-`Copy`, the parameter binding is consumed by
the move into `local`.

```
fn double_in_place(arr: i32[16]) -> i32[16]:
  mut local = arr               // arr consumed; local owns the array
  // mutate local[i] for each i
  local
```

#### 11.7.4 By-borrow parameters

A by-borrow parameter (`&T`) is a read-only handle to the caller's value
for the duration of the call. The caller retains ownership; the function
body may inspect the borrowed value but may not mutate it, consume it,
return it, or store it anywhere persistent.

By-borrow parameters allow inspection of non-`Copy` values without
forcing ownership transfer. The dominant use case is stdlib container
inspection: `length`, `contains`, `is_empty`, comparison, hashing, and
similar operations declare `&T` parameters and leave callers' bindings
untouched.

Full borrow semantics are specified in §11.9.

### 11.8 Call-Site Semantics

The caller writes a uniform call syntax regardless of whether the function
consumes or borrows its parameters:

```
let v = make_buffer()
let n = length(v)             // length declares &Vec; v survives the call
let v = push(v, 42)           // push declares Vec; v consumed, return rebinds
let m = length(v)             // length again; v still owned
```

The call-site syntax does not distinguish consume from borrow. The
function's signature is the authoritative contract: callers consult the
signature (directly or via tooling) to know what happens to their
arguments.

#### 11.8.1 Implicit borrow at call sites

When a function declares an `&T` parameter, the caller passes the binding
without any sigil — `length(v)` rather than `length(&v)`. The compiler
inserts the borrow operation automatically. The borrow is active for the
duration of the call expression.

This is intentional. Borrowing is a safe operation (read-only, no aliasing
hazards), so making it explicit at call sites would add visual noise
without informational value. Adding a sigil for the *dangerous* operation
— consumption — would similarly be noise: consumption is so common under
move semantics that requiring a marker would clutter most call sites.
Instead, the language treats both behaviors as signature-driven and lets
the compiler enforce correctness through use-after-move errors.

#### 11.8.2 Use-after-move

Using a binding after its value has been consumed is a compile error,
reported at the use site. The diagnostic identifies the call (or other
operation) that consumed the binding:

```
let v = make_vec()
let n = consume_fn(v)         // v consumed here
print(v)                       // ✗ compile error:
                               //   `v` was consumed by `consume_fn` at line 2
                               //   and is no longer accessible
```

The error is local: the compiler does not need to analyze the function's
implementation; the signature is sufficient.

#### 11.8.3 Method-call form

The method-call form `x.f(args)` is sugar for `f(x, args)` per §3.4's
uniform call syntax. The same ownership rules apply: if `f` declares its
first parameter as `&T`, the call borrows `x`; if as `T`, the call
consumes `x`.

```
let v = make_buffer()
let n = v.length()           // sugar for length(v); borrows v per signature
let m = v.length()           // borrows again; v still owned
let v = v.push(42)           // sugar for push(v, 42); consumes; rebind
```

Field access `x.field` and indexed access `x[i]` are not function calls
and do not consume regardless of any signature. They are language
primitives that read without ownership transfer.

#### 11.8.4 Refactoring impact

Changing a function's parameter from `T` to `&T` (consume to borrow) is a
*loosening* of the contract. Existing callers continue to compile:
arguments that were being consumed are now merely borrowed; the caller
retains access to the binding afterward (which is strictly more
permissive than the previous behavior).

Changing a parameter from `&T` to `T` (borrow to consume) is a
*tightening* of the contract. Callers that were using the argument after
the call now see use-after-move errors at those use sites. The errors are
local and clearly indicate which call consumed the value.

In neither direction is the refactor silent: tightening produces clear
compile errors at the affected sites; loosening produces no errors at all.

### 11.9 The Borrow Form (`&T`)

A borrow is a temporary, read-only handle to a value, scoped to a single
function call expression. Borrows are declared in function-parameter type
position with the `&` prefix:

```
fn length(v: &Vec[i32]) -> isize: ...
fn equals(a: &Vec[i32], b: &Vec[i32]) -> bool: ...
fn copy_into(src: &Vec[i32], dest: Vec[i32]) -> Vec[i32]: ...
```

The `&` is a parameter-position-only syntactic form. It does not appear at
call sites (per §11.8.1), in `let` or `mut` declarations, in record field
declarations, in return types, in trait bounds, or in any other position.
The borrow is created implicitly by the call dispatch when the function
declares a borrow parameter; the borrow is released when the call
expression finishes evaluating.

#### 11.9.1 Restrictions

A borrow may not be:

- **Stored in a binding.** `let r = ...` cannot bind a borrow. The
  language has no way to write down a "binding that holds a borrow"; the
  `&` form is not valid in `let`/`mut` declarations.
- **Returned from a function.** A function's return type is always an
  owned value or a `Copy` value. The `&` form does not appear in
  return-type position, except for the narrow carve-outs in §11.9.5
  (`Iterator::next` and stdlib-privileged borrow-returning functions).
- **Stored in a record field, enum variant payload, or tuple component.**
  Compound types contain owned values, never borrows.
- **Captured by a closure.** Closures capture by value (§11.10); borrows
  are not values that can be captured.

These restrictions collectively ensure that a borrow never outlives the
function call expression that created it. The compiler does not need to
track lifetimes across statements, across function boundaries, or across
data structures — the borrow exists only within one call expression and
is gone by the next statement.

These prohibitions admit a small set of narrow carve-outs documented in
§11.9.5 for specific structural positions (iterator-type fields,
Iterator::next returns, stdlib-privileged borrow-returning functions).
Outside those positions the prohibitions are absolute.

#### 11.9.2 Constraints during an active borrow

While a borrow of value `v` is active (i.e., during the call expression
where `&v` was passed), the owner of `v` may not:

- Move `v` (pass it by value to another function, return it, assign it to
  another binding).
- Mutate `v` (reassign through a `mut` binding, perform indexed or field
  assignment).

These constraints apply within the call expression. Once the call returns,
the borrow is released and the owner may move or mutate `v` freely.

In practice, this restriction is invisible because expressions are
evaluated sequentially. The case it forbids is multi-argument calls where
one argument borrows and another consumes:

```
fn combine(a: &Vec[i32], b: Vec[i32]) -> ...  // a borrows; b consumes
fn consume_fn(v: Vec[i32]) -> ...

let v = make_vec()
let result = combine(v, consume_fn(v))   // ✗ compile error:
                                          //   v borrowed (per combine's first parameter)
                                          //   and moved (into consume_fn) in the same expression
```

The compiler tracks within-expression borrow activity and reports
conflicts at the offending sub-expression.

#### 11.9.3 Multiple simultaneous borrows

Multiple borrows of the same value in the same expression are permitted,
because all borrows are read-only:

```
fn compare(a: &Vec[i32], b: &Vec[i32]) -> bool
fn max3(a: &Vec[i32], b: &Vec[i32], c: &Vec[i32]) -> &Vec[i32]  // illustrative; see §11.9.5 stdlib carve-out

let v = make_vec()
let r = compare(v, v)                // ✓ two borrows of v per compare's signature, both read-only
let s = max3(a, a, b)                // ✓ three borrows total per max3's signature
```

No aliasing-with-mutation hazard arises: nothing in the call expression
can mutate any of the borrowed values, by §11.9.2.

#### 11.9.4 Implicit reborrow inside function bodies

A function whose parameter is `&T` may pass that parameter to another
function expecting `&T` without any additional syntax:

```
fn length(v: &Vec[i32]) -> isize:
  count_elements(v)           // count_elements declares &Vec; reborrow is automatic

fn count_elements(v: &Vec[i32]) -> isize: ...
```

The body of `length` treats `v` as a value of type `Vec[i32]` for purposes
of reading; passing it to `count_elements` extends the borrow chain. The
compiler tracks that `v` inside `length` is a borrow (not an owned value)
and forbids operations that would consume or store it:

```
fn length(v: &Vec[i32]) -> isize:
  count_elements(v)           // ✓ reborrow
  consume_vec(v)              // ✗ consume_vec declares Vec by value;
                              //   cannot move out of borrowed v
  return v                    // ✗ cannot return a borrow as owned
  let saved = v               // ✗ cannot bind a borrow
```

#### 11.9.5 Where borrows may appear

The borrow-position list below covers both permitted and forbidden
cases; readers may treat the bullets as a complete enumeration.

`&T` is grammatically valid only in *parameter positions*. The language
recognizes six positions, each with a clear, lexically-bounded borrow
lifetime:

- **Function parameter type signatures** (this section, §11.7.4 and
  §11.9). A `&T` in a parameter signature declares that the function
  borrows that argument for the duration of the call expression.
- **For-loop iteration variables** (§12.3.3). The iteration variable is
  bound by the loop construct, fresh each iteration, immutable, and
  cannot be declared `mut`. When the iterator's `Item` type is a borrow
  type, the iteration variable is borrow-shaped for the duration of one
  iteration body.
- **For-loop iteration source expression** (§12.3.1). The expression
  between `for x in` and `:` may be a borrow expression `&v`, which
  invokes borrowing iteration via the `Iterable` trait (§12.8). The
  borrow lives for the duration of the entire for-loop expression. This
  is the only position where `&v` is an *expression* rather than a
  signature element; everywhere else, `&v` as an expression is a parse
  error.
- **Iterator type fields holding a borrow into the iteration source**
  (§12.7.4). When a user-defined iterator type satisfies the `Iterator`
  trait with `Item = &T`, the iterator's record may carry a `source: &T`
  field referencing the iteration source. The borrow's lifetime is
  bounded by the enclosing for-loop expression that holds the iterator;
  the compiler verifies that the source outlives the iterator at
  construction. This carve-out is bounded to fields of types satisfying
  `Iterator` — it does not generalize to arbitrary record fields.
- **`Iterator::next` return values** (§12.7.4). The `Iterator` trait's
  `next` method may return `(Option[&T], Self)` when the iterator's
  `Item` type is a borrow. This is the value-side counterpart to the
  iterator-type-field carve-out above: the iterator stores the borrow in
  its source field, and `next` exposes that borrow as the yielded item.
  The borrow's lifetime is bounded by the enclosing for-loop expression.
- **Stdlib-privileged borrow-returning function signatures** (§3.7.3).
  The language reserves the right to provide functions in the standard
  library whose return type is a borrow into one of their arguments — for
  example, indexed-collection accessors like `element_at(v: &Vec[T], i:
  isize) -> &T`. The returned borrow's lifetime is bounded by the
  argument-borrow's lifetime (the borrow-scope of the call expression
  that constructed the argument borrow). This carve-out applies only to
  language-privileged stdlib functions enumerated in §3.7.3; user-defined
  functions cannot declare borrow return types.

All six positions share the same lifetime discipline: the borrow lives
for the scope of one parameter-binding occurrence (one call expression,
one iteration body, or one for-loop expression respectively). The
compiler does not need lifetime parameters or cross-statement tracking;
the lifetime is determined syntactically by the enclosing position.

Outside these positions, `&T` is a parse error.

**As a type expression** (`&T` in a type-annotation position), it is
forbidden in:

- `let` and `mut` declarations: `let r: &Vec = ...` is a parse error.
- Record fields: `type Holder: data: &Vec` is a parse error. Exception:
  iterator-type fields per the position list above.
- Enum payloads: `Variant(&T)` is a parse error.
- Tuple components, *except* the tuple appearing as the return type of
  `Iterator::next` per §12.7.4: `(&i32, i32)` is generally a parse error.
- Function return types, *except* `Iterator::next` per §12.7.4 and
  stdlib-privileged borrow-returning functions per the carve-out above
  (`element_at`, etc.): `fn f(...) -> &T` is generally a parse error.
- Closure parameter types in stored closure types (deferred).
- Trait associated types, *except* `Iterator::Item` per §12.7.4:
  `type Item = &T` is generally a parse error.
- Generic-type arguments, *except* in `Option[&T]` as it appears in
  `Iterator::next`'s return per §12.7.4: `Vec[&i32]` is a parse error.

**As a value expression** (`&v` evaluating to a borrow of `v`), it is
forbidden everywhere *except* in the for-loop iteration source position
(§12.3.1):

- Function argument expressions: `f(&v)` is a parse error. Function-call
  argument syntax does not include `&`; the function's signature
  determines whether the argument is consumed or borrowed (§11.8.1).
- Binding right-hand sides: `let r = &v` is a parse error.
- Return expressions: `return &v` is a parse error.
- Record/tuple/enum construction: `Holder(field: &v)` is a parse error.
- Closure capture expressions: borrows cannot be captured.

The narrow exceptions for the `Iterator` trait (above) and for the
iteration source expression position (§12.3.1) exist because both flow
directly into the for-loop iteration variable position; the borrow's
lifetime is bounded by the iteration body or the for-loop expression in
exactly the same way function-parameter borrows are bounded by the call
expression. The exceptions are bounded; they do not generalize to
arbitrary user code.

This restriction keeps the borrow model trivially sound without lifetime
parameters or cross-statement tracking.

#### 11.9.6 Borrows of `Copy` values

A `Copy` value may also be passed by borrow rather than by value if the
function declares `&T`. The behavior is identical in effect — neither
form consumes the caller's binding — and the choice is the function
author's. Borrow declarations on `Copy` types are unusual but legal; the
caller cannot tell the difference at the call site.

The motivation for declaring `&T` even when `T` is `Copy` is uniform API
shape: a function generic over `T: SomeTrait` may want to take `&T` so
the behavior is consistent across `Copy` and non-`Copy` instantiations.

### 11.10 Closures and Capture

A closure is an anonymous function that may capture values from its
enclosing scope. Ductus's closures capture *by value*: each captured
value is stored inside the closure at the moment of definition.

#### 11.10.1 Captures must be `Copy`

Every value captured by a closure must have a `Copy` type. The captured
value is a snapshot of the source value at definition time; the closure
holds an independent copy.

```
let gain: f32 = 1.5
let process = |sample: f32| sample * gain    // captures gain (f32, Copy) ✓
```

For non-`Copy` source values, capture is a compile error:

```
let buf = make_buffer()                       // Vec[f32], non-Copy
let closure = || sum(buf)                     // ✗ compile error:
                                              //   cannot capture non-Copy value `buf`
```

Non-`Copy` values flow through closures as arguments rather than captures:

```
let closure = |b: &Vec[f32]| sum(b)          // closure takes a borrow argument
let total = closure(buf)                      // caller passes buf; borrow inserted per signature
```

#### 11.10.2 Capture granularity

The compiler captures the minimal set of subvalues the closure body
references. For a body that reads a single field of a larger record, only
that field is captured — provided the field's type is `Copy`:

```
let contact = Contact(first_name: "Alice", age: 30, ...)
let closure = || contact.age + 1              // captures contact.age (i32, Copy)
                                              // contact stays in outer scope, fully usable
```

If a captured subvalue's type is not `Copy`, the capture fails regardless
of whether the root binding is `Copy`. The constraint applies to the
captured value's type, not the root binding's type.

#### 11.10.3 Captures from `let` only

Closures may capture from `let` bindings. They may not capture from `mut`
bindings:

```
let stable = 5
let closure_a = || stable + 1                 // ✓ capture from let

mut counter = 0
let closure_b = || counter + 1                // ✗ compile error:
                                              //   cannot capture from `mut` binding `counter`
```

The forbid prevents a footgun: closures capture by value at definition
time (a snapshot), but users naming a `mut` binding might intuitively
expect the closure to see live updates. Forbidding the capture forces the
user to make the snapshot explicit:

```
mut counter = 0
counter = compute_initial()
let snapshot = counter                        // explicit snapshot via let
let closure = || snapshot + 1                 // captures snapshot (Copy)
counter = counter + 1                         // mut continues to evolve
                                              // closure still sees the snapshot value
```

For closures that must track live updates of changing state, the reactive
system is the appropriate mechanism. The reactive system is specified in
§13 (see §13.2 for reactive cell declarations).

#### 11.10.4 Body unrestricted

Within a closure body, all the usual function-body rules apply. The body
may declare local `mut` bindings, call functions (consuming arguments as
their signatures dictate), construct new values, perform iteration
(once specified), and so on. The capture-must-be-`Copy` restriction
applies only to the closure's captured environment, not to anything the
body does internally.

```
let scale: f32 = 2.0                          // Copy capture
let process = |raw: Vec[f32]| -> Vec[f32]:
  mut local = raw                              // mut local; allowed inside closure body
  apply_scale_in_place(local, scale)           // internal work; captures untouched
  local
```

#### 11.10.5 Borrows cannot be captured

A borrow is not a value (per §11.9). Closures capture values; therefore
closures cannot capture borrows. There is no `&T` form usable in a
capture context.

A closure body may receive borrows as *arguments* (its parameter list may
include `&T` parameters), but it cannot retain borrows from outside its
parameter list.

### 11.11 Indexed and Field Assignment

Assignment through a `mut` binding to a field or array element is
permitted:

```
mut r = make_record()
r.field = new_value          // ✓ field assignment

mut arr = make_array()
arr[5] = 1.5                  // ✓ indexed assignment
```

The root binding (`r`, `arr`) must be declared `mut`. The field or
element being assigned must itself be of a type compatible with the
assigned value, per the standard type-check rules.

Field and indexed assignment desugar to operator-trait method calls (the
exact traits — `FieldAssign`, `IndexAssign`, or analogous — are stdlib
concerns specified outside this document). The desugaring preserves the
single-writer invariant: the assignment is a mutation through the `mut`
binding only; no other binding to the same underlying value can exist
while the mutation occurs (borrows would block it per §11.9.2; aliased
ownership is impossible by construction).

Reading a field or element from any binding (whether `let` or `mut`) is
unrestricted (§11.3.1).

#### 11.11.1 Whole-value reassignment

A `mut` binding may be reassigned entirely:

```
mut buf = make_buffer()
buf = make_other_buffer()    // replaces the buffer; old one dropped
```

This drops the previous value and binds the new value. The new value is
moved into the binding (consumed from its source).

### 11.12 Interaction with Records, Enums, and Newtypes

The compound types of §6 interact with mutability as follows:

- A record, enum, or newtype may itself be `Copy` if it carries
  `@derive(Copy)` and all its fields/payloads are `Copy`.
- Mutability is purely a property of the binding (per §11.2), not of the
  type. A type does not declare "this is mutable"; specific bindings to
  values of the type may be declared `mut`.
- A record's fields, an enum variant's payload, and a newtype's wrapped
  value may all be assigned through a `mut` binding to the containing
  value, provided the field/payload/wrapped type permits assignment.

Records (§6.1) explicitly forbid `fn` declarations in their bodies; this
forbid does not extend to disallowing `mut` interaction. A function
elsewhere that holds a `mut` binding to a record may freely assign its
fields.

#### 11.12.1 Smart constructors and `mut`

The `public(private)` constructor pattern (§6.1.7, §6.3.4) restricts
construction to the type's defining module. This restriction interacts
naturally with `mut`: any code holding a `mut` binding to such a type can
still mutate its fields (subject to field visibility per §10.7); the
restriction is only on initial construction, not on subsequent mutation.

For types where post-construction mutation should also be restricted, the
appropriate mechanism is field-level visibility (`private field_name:
T`), which prevents external code from naming the field in an assignment
expression.

### 11.13 Interaction with the Trait System

Trait method signatures may declare borrow parameters identically to free
functions:

```
trait Length:
  fn length(value: &Self) -> isize

fulfill Length for Vec[i32]:
  fn length(value: &Vec[i32]) -> isize:
    ...
```

The `&Self` in the trait declaration becomes `&Vec[i32]` (or whatever the
implementing type is) in each `fulfill` block, by the standard `Self`
substitution rule of §3.1.1.

Trait dispatch (§3.4) is unaffected by ownership semantics. Whether a
trait method consumes or borrows its receiver depends on the trait
method's signature; callers write the same uniform-call syntax regardless.

Trait objects (§5.2, `dyn T`) may invoke methods that declare `&T`
parameters. The borrow lifetime remains call-scoped as for direct calls.

### 11.14 Interaction with Reactivity

The reactive system is specified in §13. The interaction with local
mutability follows two principles, recorded here for
forward-compatibility:

- **Reactive expressions (`derived` and analogous) are pure-evaluated.**
  A `derived` expression's body runs as a pure function of its inputs
  each time inputs change. The body may invoke ordinary functions that
  use `mut` internally; the body itself produces a value that enters the
  reactive graph.

- **Values entering the reactive graph become external state.** Once a
  value is bound into the reactive system as a signal, derived, or
  reactive store, it is no longer the property of any single function's
  scope. External state is immutable per §11.1's "nothing outside a
  function body is mutable" principle; reactive values are updated only
  through the reactive system's defined update mechanisms, never through
  `mut` assignment.

The reactive boundary is one of the "global" scopes referenced in
§11.1's principles. The full specification of how values cross this
boundary is given in §13.12 (the reactivity boundary).

---

*End of §11.*

---

## 12. Iteration and Loops

This section specifies the language's iteration constructs: integer ranges,
the `for`-loop (in both consuming and borrowing forms), the `while`-loop,
the `break` and `continue` statements, loop expression semantics with the
`else:` clause, and the `Iterator`, `Iterable`, and `IntoIterable` traits
that underlie all iteration.

Loops are the necessary complement to local mutability (§11): without
bounded iteration, indexed buffer construction and accumulation patterns
require recursion, which is unusable for the workloads (audio DSP, image
processing, numerical kernels) that motivate the local mutability model.
This section completes the imperative-control story for performance-
sensitive code while keeping the rest of the language pure and functional.

### 12.1 Design Principles

Loops in Ductus follow three guiding rules:

**Iteration is trait-driven.** A `for` loop dispatches through the
`IntoIterable` trait (consume form, default) or the `Iterable` trait
(borrow form, explicit `&v`) to obtain an `Iterator`, then dispatches
through the `Iterator` trait to produce successive values. There is no
built-in iteration logic specific to particular types; all iteration
goes through the trait protocol. Users may extend iteration to their
own types by implementing either or both traits.

**Loops are expressions.** Both `for` and `while` produce values. The
value is determined by the body's `break` statements and an optional
`else:` clause (§12.6). Loops that are not used in an expression context
produce unit; loops that are used in an expression context obey the
value-shaping rules of §12.6.

**Mutation discipline is preserved.** Loop bodies are ordinary function
body fragments. They can mutate `mut` bindings declared inside or outside
the loop, perform indexed and field assignment through `mut` bindings,
and call functions per the ownership rules of §11. Under the borrow
form, the borrow checking rules of §11.9 apply: while a collection is
being iterated, the collection is borrowed and may not be moved or
mutated through its owner. Under the consume form, the collection is
consumed at loop entry and no longer accessible to the surrounding scope.

### 12.2 Ranges

A *range* is a value representing a sequence of integers. The range
expression syntax is `start..end`, where `start` and `end` are integer
expressions:

```
0..10                          // integers 0, 1, 2, ..., 9
1..100                         // integers 1, 2, ..., 99
n..(n + size)                  // dependent on n and size
```

Ranges are half-open and exclusive on the upper bound: `start..end`
contains integers `i` such that `start <= i < end`. To iterate up to
and including some value `N`, write `start..(N + 1)`. There is no
inclusive-range form (`..=`) in v1.

Ranges are values of type `Range[T]`, where `T` is the integer type of
`start` and `end` (the two operands must have the same integer type;
mixed-kind operands require explicit conversion). Ranges may be bound,
passed to functions, returned, and used like any other value:

```
let r = 0..1024              // r: Range[i32], by default integer placeholder
fn process_range(r: Range[i32]) -> isize: ...
```

`Range[T]` implements both `IntoIterable` (§12.9) and `Iterable`
(§12.8). The consume form `for i in 0..N` dispatches through
`IntoIterable`; the borrow form `for i in &(0..N)` dispatches through
`Iterable`. Since `Range[T]` is `Copy`, the two forms are functionally
indistinguishable from the user's perspective — there is nothing to
preserve on the source side either way. The implementations yield
successive integers starting from `start` and stopping before `end`.
If `start >= end`, the range is empty and yields no values.

#### 12.2.1 Step

The v1 range syntax has no step parameter. Iteration always advances by
one. To iterate with a different stride, the user writes the arithmetic
explicitly:

```
for i in 0..(N / 2):
  let actual_i = i * 2          // 0, 2, 4, ..., N-2
  process(actual_i)
```

A step-aware range form may be added in a future version of the language;
for v1, the explicit form is the supported pattern.

#### 12.2.2 Range bounds and overflow

A range's bounds are evaluated once at the point the range value is
constructed. Subsequent mutation of any variable used in those expressions
does not affect the range:

```
mut n = 10
let r = 0..n
n = 100
for i in r:                   // iterates 0..10, not 0..100
  ...
```

The bound expressions must produce integer values. Per §4.6.1, overflow
in the bound expressions traps at construction.

#### 12.2.3 Negative ranges and empty ranges

A range whose `start >= end` is empty. `for i in 5..3:` produces no
iterations and (in expression context) goes directly to the `else:`
clause if present, or produces the natural-completion value per §12.6
otherwise.

Ranges with negative starts and ends work normally if `T` is signed:

```
for i in -10..10:             // i: i32 (default); -10, -9, ..., 9
  ...
```

For unsigned `T`, negative literals are rejected at the value-fits-check
per §2.4.3.

### 12.3 The `for` Loop

The `for` loop iterates over the values produced by an iteration source.
The source can be passed in two forms, which select between consuming
and borrowing iteration:

```
for x in iterable:           // consumes iterable (default)
  body

for x in &iterable:          // borrows iterable
  body
```

Consume-form `for x in v` is the default because ownership transfer is
the language's default for any value passed into a sub-scope (parallel
to function calls per §11.7). Borrow-form `for x in &v` explicitly opts
out of ownership transfer when the source must remain usable after the
loop. This matches the parameter rule (`fn f(x: T)` consumes, `fn f(x:
&T)` borrows) but operates on the loop expression rather than a function
signature.

#### 12.3.1 Evaluation

The iteration source expression is evaluated once at loop entry.

**Consume form** (`for x in v:`):

1. The compiler invokes `IntoIterable::consuming_iterator(v)` (§12.9).
   This consumes the binding `v`; the underlying value is moved into
   the iterator.
2. The iterator is held in an internal `mut` binding for the loop's
   duration.
3. Each iteration step calls `Iterator::next` (§12.7), receiving
   `(Option[Item], NewIter)`. Under consuming iteration, `Item` is
   always an owned type per §12.9.2.
4. The internal binding is reassigned to `NewIter`.
5. If the option is `Some(value)`, binds `value` to the iteration
   variable `x` and runs the body.
6. If `None`, the loop exits.
7. When the loop exits (natural completion, `break`, or enclosing
   function return), the iterator is dropped. Any elements not yet
   yielded are dropped per their `Drop` semantics. The original source
   binding `v` is consumed; it cannot be referenced after the loop.

**Borrow form** (`for x in &v:`):

1. The compiler evaluates `&v` as a borrow expression and invokes
   `Iterable::iterator(&v)` (§12.8). The borrow lives for the duration
   of the for-loop expression.
2. The iterator is held in an internal `mut` binding for the loop's
   duration.
3. Each iteration step calls `Iterator::next`, receiving
   `(Option[Item], NewIter)`. Under borrowing iteration, `Item` may be
   a borrow type (`&T`) per §12.7.4; the iteration variable's type
   follows from `Item`.
4. The internal binding is reassigned to `NewIter`.
5. If the option is `Some(value)`, binds `value` to the iteration
   variable `x` and runs the body.
6. If `None`, the loop exits.
7. When the loop exits, the iterator and the borrow of `v` are
   released. `v` is unchanged and remains owned by the original
   binding.

The `&v` form is the only place in the language where `&` appears as
a value expression rather than a type annotation. Its lifetime is
bounded by the for-loop expression, requiring no annotations or
cross-statement tracking (§11.9.5).

**Iteration source that is already a borrow:** when the iteration
source expression is of a borrow type (because the binding being
referenced is itself a borrow — e.g., a function parameter typed `&T`,
or an iteration variable from an outer loop typed `&T`), the for-loop
dispatches to `Iterable` directly. No explicit `&` is needed because
the value is already a borrow:

```
fn sum(samples: &Vec[f32]) -> f32:
  mut total: f32 = 0.0
  for s in samples:            // samples is already &Vec; Iterable dispatch
    total = total + s
  total
```

Writing `&samples` in this position is a compile error — `&` may only
be applied to owned values, not to expressions that already evaluate
to a borrow. (The grammar accepts `&samples` syntactically; the type
checker rejects it once it determines `samples` is of borrow type.)
The compiler dispatches based on the type of the iteration source:
owned types use `IntoIterable` (consume); borrow types use `Iterable`
(no consume possible, since borrows can't be consumed).

This means the rule "consume by default" applies to owned bindings.
For borrowed bindings, iteration is necessarily through `Iterable`
because the language cannot move out of a borrow (§11.9). Borrowed
sources iterate as if `&` were written, without requiring the user to
add it.

#### 12.3.2 The iteration variable

The iteration variable `x` is bound fresh on each iteration. It is a
`let`-style binding: immutable, with the iterated `Item` type. Reassigning
`x` within the body is a compile error.

`x` cannot be declared `mut`. The form `for mut x in iterable:` is a
parse error. This is consistent with §11.7.2's prohibition on `mut`
parameters — the iteration variable is bound by the loop construct, not
by user declaration, and follows the same rule.

If the body needs a mutable per-iteration value, it rebinds:

```
for x in 0..N:
  mut local = x
  local = local * 2
  process(local)
```

#### 12.3.3 Iteration variable type

The iteration variable's type is `Iter::Item`, where `Iter` is the
iterator type produced by the dispatch (either `IntoIterable::Iter`
under consume form, or `Iterable::Iter` under borrow form). The Item
type depends on both the iterable's element type and which form the
loop uses.

The iteration variable is one of the six valid borrow-bearing
positions per §11.9.5: bound by the loop construct, fresh each
iteration, immutable, cannot be declared `mut`. Its borrow lifetime
(when borrow-typed) is the duration of one iteration body. The compiler
tracks this without requiring lifetime annotations.

**Consume form, Copy element type** (`for sample in buf:` where `buf:
f32[1024]`): the iteration variable is an owned Copy value. The body
uses it freely.

```
let buf: f32[1024] = make_block()
mut sum: f32 = 0.0
for sample in buf:                  // sample: f32, owned (Copy)
  sum = sum + sample
// buf is consumed; cannot be used after the loop
```

**Consume form, non-Copy element type** (`for r in records:` where
`records: Vec[Record]`): the iteration variable is an owned `Record`.
Each iteration moves one record out of the consumed Vec's storage. The
body has full ownership of `r` — it can be moved into bindings, passed
to consuming functions, stored elsewhere.

```
mut destinations = make_collection()
let records: Vec[Record] = make_records()
for r in records:                   // r: Record, owned
  destinations = destinations.push(r)   // ✓ move r into destinations
// records is consumed; destinations contains the records' owned values
```

**Borrow form, Copy element type** (`for sample in &buf:` where `buf:
f32[1024]`): the iteration variable is a Copy value, identical in
behavior to the consume-form Copy case. The borrow form's only
observable difference for Copy elements is that `buf` survives the loop.

```
let buf: f32[1024] = make_block()
mut sum: f32 = 0.0
for sample in &buf:                 // sample: f32, Copy
  sum = sum + sample
process_further(buf)                // ✓ buf still owned
```

**Borrow form, non-Copy element type** (`for r in &records:` where
`records: Vec[Record]`): the iteration variable is `&Record` — a
borrow into the source's storage. The body can read fields, call
methods that take `&T`, compare, inspect, but cannot move `r` into a
binding, pass it to a consuming function, or store it past the
iteration body.

```
let records: Vec[Record] = make_records()
for r in &records:                  // r: &Record (Record is non-Copy)
  print(r.first_name)                // ✓ read access
  process_borrow(r)                  // ✓ if process_borrow takes &Record
  consume(r)                          // ✗ compile error: cannot move out of borrow
  let saved = r                       // ✗ compile error: cannot bind a borrow
// records still alive
process_further(records)
```

The borrow form's non-Copy case is what makes "iterate to inspect a
non-Copy collection" possible without paying clone cost. The iterator's
`Item` is `&Record`; the iteration variable inherits this; the borrow
is bounded by the iteration body. See §12.7.4 for how the Iterator
trait handles borrow-typed Items.

#### 12.3.4 Body scope

The body executes in a fresh nested scope each iteration. Bindings
declared inside the body are dropped at the end of each iteration. The
iteration variable `x` is in scope only within the body.

Bindings declared OUTSIDE the loop are in scope inside the body. They
persist across iterations and can be mutated if declared `mut`:

```
mut total: f32 = 0.0
for sample in samples:
  total = total + sample
print(total)                  // accumulated sum
```

This is the accumulator pattern.

#### 12.3.5 Move restrictions inside the body

Per §11, moving a value out of an outer binding inside the loop body
causes the binding to be consumed. If the loop runs more than once and
the body references that binding again, the second iteration produces a
use-after-move compile error.

```
let v = make_vec()
for i in 0..10:
  consume(v)                 // ✗ compile error: v consumed; subsequent iterations
                             //   would attempt to use already-moved v
```

The compiler detects this conservatively: any move of an outer binding
inside a loop body is reported as a potential use-after-move at the move
site, with a note explaining that the loop may execute multiple times.

To consume a value inside a loop body, the user can:

- `.clone()` the value per iteration (explicit cost).
- Restructure to consume after the loop, not inside.
- Move the value into the loop with a single-iteration loop (rare).

#### 12.3.6 Mutation of the iterated source

Under the borrow form (`for x in &v:`), the iteration source is
borrowed for the duration of the loop. Per §11.9.2, the owner may not
mutate the value through its binding while a borrow is active:

```
mut v = make_vec()
for x in &v:
  v[0] = 5                   // ✗ compile error: v is borrowed for iteration
```

This prevents iterator invalidation. The borrow ends when the loop
exits, after which the owner may freely mutate or move the value.

Under the consume form (`for x in v:`), the question doesn't arise:
`v` is consumed at loop entry; the binding doesn't exist inside the
loop body. Attempting to use `v` inside the body would be a
use-after-move error, not a borrow conflict.

### 12.4 The `while` Loop

The `while` loop repeatedly evaluates a condition and runs its body so
long as the condition is true:

```
while condition:
  body
```

#### 12.4.1 Evaluation

On each iteration:

1. The condition expression is evaluated. It must produce a value of type
   `bool`.
2. If the result is `true`, the body executes.
3. If the result is `false`, the loop exits.

The condition is re-evaluated before each iteration, including the first.
A loop whose condition is `false` at entry never executes its body.

#### 12.4.2 Idiomatic uses

The `while` loop is the right tool when the number of iterations is not
known at loop entry. Examples include polling, fixed-point computation,
state-machine progression, and consuming streaming inputs:

```
mut converged = false
mut value = initial_guess
while not converged:
  let next = update(value)
  converged = approx_equal(next, value)
  value = next

mut state = State::Initial
while state is not State::Done:
  state = step(state)
```

For "loop forever" patterns, `while true:` is the idiomatic form. There
is no separate `loop` keyword.

#### 12.4.3 Move restrictions

The same move-inside-loop rule from §12.3.5 applies. A non-`Copy` outer
binding consumed inside a `while` body produces a use-after-move error if
the loop may iterate more than once. The condition expression is also
subject to the same rule.

### 12.5 `break` and `continue`

The `break` statement exits the innermost enclosing loop. The `continue`
statement skips to the next iteration of the innermost enclosing loop.

```
for i in 0..N:
  if should_skip(i):
    continue                  // skip the rest of this iteration; go to next i
  if should_stop(i):
    break                     // exit the loop entirely
  process(i)
```

#### 12.5.1 `break` with value

In expression context (§12.6), `break` may carry a value:

```
break expr
```

The expression's value becomes the loop's expression value. The body's
`break value` sites must all produce values of compatible types, and (if
an `else:` clause is present) must agree with the else clause's type.

The plain `break` form (without value) is equivalent to `break ()` —
exiting with the unit value. A loop body that mixes `break` and
`break value` with a non-unit value is a type error.

#### 12.5.2 `continue` carries no value

`continue` does not produce a value. It is a control-flow statement only,
advancing the loop to its next iteration. The loop's expression value is
determined by `break` and `else:`, not by `continue`.

#### 12.5.3 Innermost loop only

`break` and `continue` always target the innermost enclosing loop. There
is no label syntax for targeting outer loops in v1. To exit a nested
loop construct, the user refactors to use a flag variable or extracts the
inner loop into a function that returns early.

```
fn find_in_grid(g: &Grid, target: &Cell) -> Option[(isize, isize)]:
  for row in 0..g.rows:
    for col in 0..g.cols:
      if g.get(row, col) is target:
        return Some((row, col))    // returns from the function, exiting both loops
  None
```

#### 12.5.4 `break` and `continue` outside loops

A `break` or `continue` outside a loop is a parse error.

### 12.6 Loop Expressions and the `else:` Clause

Both `for` and `while` loops produce values when used in expression
context. The value is determined by the body's `break value` sites and an
optional `else:` clause.

#### 12.6.1 The `else:` clause

A loop may have an optional `else:` clause attached to its body:

```
for i in iterable:
  body
else:
  natural_completion_value

while condition:
  body
else:
  natural_completion_value
```

The `else:` clause's expression is evaluated exactly when the loop
completes *naturally* — meaning iteration exhausts (for `for` loops) or
the condition becomes false (for `while` loops). The `else:` clause is
*not* evaluated when the loop exits via `break` or via an enclosing
function return.

The `else:` keyword is reused from `if`/`match` constructs but has
different semantics here. A reader should understand `else:` on a loop as
"otherwise, the loop completed naturally and this is the value."

#### 12.6.2 Loop expression type

The loop expression's type is determined by the combination of `break
value` sites in the body and the presence/absence of an `else:` clause:

| Body has `break value` | `else:` clause | Loop expression type       |
|------------------------|----------------|----------------------------|
| No                     | absent         | `()` (unit)                |
| No                     | present        | type of `else:` expression |
| Yes                    | absent         | `Option[T]`                |
| Yes                    | present        | `T`                        |

where `T` is the unified type of all `break value` sites (and the
`else:` clause, when present).

**Note on `never`-unification:** if the body provably never completes
naturally (every path produces a `break value`, `return`, `panic`, or
other diverging operation), the natural-completion arm is unreachable
and unifies with the break-value type via §8.2.2. In this case the
loop expression's type collapses to `T` regardless of the `else:`
clause's presence. See §12.6.4.

##### Without `break value`, without `else:`

The loop produces unit. This is the statement form.

```
for i in 0..N:
  process(i)
                              // expression value: () (unit)
```

##### Without `break value`, with `else:`

The loop produces the `else:` clause's value. The body never produces a
value via break, so the only path to a loop value is natural completion
through `else:`:

```
let summary = for sample in samples:
  process(sample)
else:
  count_of_samples            // always reached after natural completion
                              // expression value: count_of_samples
```

This form is unusual but consistent. It is most useful when the body has
significant side effects (mutations, function calls) and the user wants
the loop to also yield a summary value.

##### With `break value`, without `else:`

The loop produces `Option[T]` where `T` is the type of the `break value`
expression. The language auto-wraps each `break value` as `Some(value)`;
natural completion produces `None`. The user writes the bare value (not
`Some(...)`).

```
let found = for item in &items:
  if matches(item):
    break item.id              // borrow form: items survives the loop
                               // auto-wrapped to Some(item.id)
                               // expression type: Option[ItemId]
```

For the find-first pattern, the user typically wants `Some(item)` from
the break and `None` from natural completion (no match). With this
shape, the loop's expression type is `Option[Item]`, and the user
match-decides on the result.

##### With `break value`, with `else:`

The loop produces `T`. The `break value` sites and the `else:` clause
all produce values of the same type:

```
let answer = for n in &numbers:
  if is_special(n):
    break n
else:
  -1                          // fallback when no n is special
                              // expression value: i32 (n is Copy)
```

This form is typical when the user wants a guaranteed value without
unwrapping an Option.

#### 12.6.3 Type unification

All `break value` expressions in a single loop body must produce values of
the same type (or be unifiable). When an `else:` clause is present, its
expression must produce a value of the same type. If types cannot be
unified, the compiler reports a type error at the conflicting break or
else site.

```
for i in 0..N:
  if cond_a: break 42
  if cond_b: break "hello"      // ✗ type error: i32 vs string
else:
  ...
```

#### 12.6.4 The `never` type and unreachable completions

If the body provably never completes naturally — for instance, every
path through the body produces a `break value`, or terminates via
`return`, `panic`, or other diverging operation — the natural-completion
case is unreachable. The compiler may use the `never` type (§8.2.2) to
unify the unreachable case with any other type.

```
let value = for i in 0..N:
  if condition(i):
    break i
  else:
    panic("unexpected")        // diverges; never type
                                // expression value: i32 (from break)
                                // no else: clause needed; natural completion unreachable
```

#### 12.6.5 `continue` and the expression value

`continue` does not contribute to the loop's expression value. It advances
to the next iteration without producing a value. The loop's value is
determined by `break value` and `else:` only.

### 12.7 The `Iterator` Trait

`Iterator` is the stdlib trait for types that produce a sequence of values
on demand:

```
trait Iterator:
  type Item
  fn next(iter: Self) -> (Option[Item], Self)
```

The `next` method takes the iterator by value, advances its internal
state, and returns both the next item (or `None` if the iteration is
complete) and the advanced iterator. The caller binds the returned
iterator for the next call.

#### 12.7.1 Why the tuple return

The trait method returns `(Option[Item], Self)` because the iterator's
internal cursor state must be mutated across calls, but the language has
no `&mut` parameter form (§11.9) and forbids `mut` on parameters
(§11.7.2). Under these constraints, the only way to advance an
iterator's state across a method call is to take the iterator by value
(consume it) and return the advanced version alongside the item.

The for-loop desugaring (§12.3.1) hides this verbosity from user code:
the user writes `for x in v:` and the compiler emits the rebind pattern
implicitly.

#### 12.7.2 Linear-ownership optimization

Because the for-loop's iterator binding is owned exclusively by the loop
and is reassigned only by the loop's internal desugaring, the iterator's
ownership is *linear* (single owner at every moment, no aliasing). The
compiler recognizes this pattern and emits in-place cursor mutation —
equivalent to the machine code produced for `&mut self` methods in
languages with mutable references.

Specifically, when:

1. The iterator type is statically known (monomorphized per §2.3),
2. The iterator binding is held in a single `mut` location (the
   for-loop's internal binding),
3. The `next` call's return value's `NewIter` component is
   immediately destructured and rebound to that same `mut` location,
   in a single statement, with no other reference to the consumed
   binding between the `next` call and the rebind.

Condition 3 holds by construction for the for-loop's internal
desugaring: the desugaring emits one statement that calls `next`,
destructures the returned tuple via pattern match, and rebinds the
iterator location to `NewIter` — all in one expression with no other
references possible.

When the three conditions hold, the compiler compiles the call as:
pass a pointer to the iterator's state, mutate the cursor in place,
return only the item value in registers. The "consumed" and "returned"
iterator are the same memory location; no copy occurs.

This optimization is a *required* property of conforming implementations,
not an optional optimization. The tuple-return trait shape is the
source-level pattern; the linear-ownership compilation is the performance
guarantee.

#### 12.7.3 Implementing `Iterator`

A user-defined iterator implements `Iterator` by declaring the `Item`
associated type and the `next` method:

```
type SquareIter:
  next_value: i32
  limit: i32

fulfill Iterator for SquareIter:
  type Item = i32
  fn next(iter: SquareIter) -> (Option[i32], SquareIter):
    mut local = iter
    if local.next_value >= local.limit:
      (None, local)
    else:
      let value = local.next_value * local.next_value
      local.next_value = local.next_value + 1
      (Some(value), local)
```

This implementation receives the iterator by value, rebinds to a local
`mut` binding (per §11.7.3), mutates the cursor, and returns the result
alongside the (updated) iterator.

#### 12.7.4 Borrow-typed `Item`

The `Item` associated type may be a borrow type (`&T`) when the iterator
yields non-Copy elements from a source it borrows. This is one of the
narrow positions where `&T` is grammatically valid as a type expression
(see §11.9.5 for the complete list); the exception is bounded to the
`Iterator` trait and is justified by `Item` flowing into the for-loop
iteration variable position (§11.9.5, §12.3.3).

When `Item = &T`, the `next` method's return type becomes
`(Option[&T], Self)`. The `&T` appears inside `Option` and inside the
tuple, both of which are normally borrow-forbidden positions; the
exception applies specifically to this trait's `next` return.

User-defined borrow-yielding iterators store a borrow into the iteration
source as a field of the iterator type. This is one of the narrow `&T`
positions permitted per §11.9.5; the lifetime is bounded by the for-loop
expression.

A borrow-yielding iterator implementation:

```
type VecIter[T]:
  source: &Vec[T]                   -- borrow into the iteration source
  cursor: isize

fulfill Iterator for VecIter[Record]:
  type Item = &Record               // yields borrows of Record elements
  fn next(iter: VecIter[Record]) -> (Option[&Record], VecIter[Record]):
    mut local = iter
    if local.cursor >= local.source.length():
      (None, local)
    else:
      let element_ref = local.source.element_at(local.cursor)   // returns &Record
      local.cursor = local.cursor + 1
      (Some(element_ref), local)
```

`local.source.element_at(local.cursor)` calls a stdlib-privileged
borrow-returning function per §11.9.5 (the §3.7.3 carve-out for
borrow-return signatures). The returned `&Record` is bounded by the
source borrow's lifetime.

The trait declaration itself is unchanged:

```
trait Iterator:
  type Item
  fn next(iter: Self) -> (Option[Item], Self)
```

`Item` is unconstrained in the trait declaration. Implementations may
declare `Item` as `T` (owned) or `&T` (borrow). The choice depends on
which trait (`Iterable` or `IntoIterable`) provides the iterator and
on the source's element type:

- Iterators from `IntoIterable::consuming_iterator` always yield owned
  Item types. Each `next` call moves one element out of the iterator's
  internal storage (which holds the consumed source's buffer). Item is
  `T` regardless of whether T is Copy.
- Iterators from `Iterable::iterator` choose based on element type:
  Copy elements yield owned values (`Item = T`); non-Copy elements
  yield borrows (`Item = &T`).

The borrow's lifetime is bounded by the iteration body. The compiler
checks that the source value (the Vec, the Record array, etc.) is not
moved or mutated while iteration is active — same rule as §11.9.2 for
function-parameter borrows, applied per iteration.

### 12.8 The `Iterable` Trait

`Iterable` is the stdlib trait for types that can produce an iterator:

```
trait Iterable:
  type Iter: Iterator
  fn iterator(value: &Self) -> Iter
```

The associated type `Iter` is the iterator type produced; it must itself
implement `Iterator`. The method `iterator` takes a borrow of the source
and returns an iterator that will yield the source's elements.

#### 12.8.1 Method name

The method is named `iterator`, not `iter`. The language convention is
to prefer full names over abbreviations (§1.4 and following). Stdlib and
user code use the full name throughout.

#### 12.8.2 Borrow lifetime

The `iterator` method's parameter is `&Self`. The iterator's lifetime is
bounded by the borrow's scope — meaning, for a for-loop, the lifetime of
the loop expression itself. This is enforced by the same call-scoped
borrow rules of §11.9: while the for-loop is running, the source value
is borrowed and may not be mutated through its owner.

The `Iterable` trait is invoked by the *borrow form* of the for-loop:
`for x in &v:` dispatches through `Iterable::iterator(&v)`. The consume
form `for x in v:` dispatches through `IntoIterable` (§12.9) instead.

#### 12.8.3 Implementing `Iterable`

A user-defined container implements `Iterable` by declaring the iterator
type and the construction method:

```
type DataPoints:
  values: f32[256]
  count: isize

fulfill Iterable for DataPoints:
  type Iter = DataPointsIter
  fn iterator(d: &DataPoints) -> DataPointsIter:
    DataPointsIter(...)        // construct iterator over d's data
```

The `for x in &d` syntax then dispatches to this implementation
automatically.

### 12.9 The `IntoIterable` Trait

`IntoIterable` is the stdlib trait for types that can be *consumed* to
produce an iterator. The source value is moved into the iterator;
elements are yielded as owned values.

```
trait IntoIterable:
  type Iter: Iterator
  fn consuming_iterator(value: Self) -> Iter
```

The associated type `Iter` is the iterator produced; it must itself
implement `Iterator`. The method `consuming_iterator` takes the source
by value (consumes it) and returns an iterator that owns the source's
storage.

#### 12.9.1 Method name and dispatch

The method is named `consuming_iterator`. The full name signals that
ownership transfers — the source is gone after the call. The convention
follows §12.8.1 (full names over abbreviations).

The `IntoIterable` trait is invoked by the *consume form* of the
for-loop: `for x in v:` dispatches through
`IntoIterable::consuming_iterator(v)`. The source `v` is consumed at
loop entry; after the loop, the binding `v` is no longer usable per the
ownership rules of §11.

#### 12.9.2 Item type and ownership flow

Under `IntoIterable`, the iterator yields owned `Item` values directly.
For non-Copy `Item` types, each `next` call physically moves one element
out of the iterator's internal storage (which holds the source's
buffer). For Copy `Item` types, each `next` call yields a copy of the
element.

The iteration variable in the for-loop is bound to the yielded value
with full ownership. The body may move it into another binding, pass
it to consuming functions, store it elsewhere — anything an owned
value supports.

```
mut destinations = Vec::new()
let records: Vec[Record] = make_records()
for r in records:                       // consume form; r: Record (owned)
  if r.is_valid():
    destinations = destinations.push(r) // ✓ move r into destinations
                                         // (the predicate r.is_valid() reads via &Record
                                         //  borrow, available because methods can declare &T)
// records consumed; destinations holds the valid records
```

#### 12.9.3 Partial consumption and Drop

If the loop exits via `break` (or via an enclosing function return)
before exhausting the iterator, elements at positions not yet yielded
remain inside the iterator's internal storage. When the iterator is
dropped (at loop exit), the remaining elements are dropped per their
`Drop` semantics, and the underlying buffer is released.

The exact `Drop` mechanism for non-Copy types is specified in §14.8.
For Copy types, drop is a no-op.

#### 12.9.4 Implementing `IntoIterable`

A user-defined container implements `IntoIterable` by declaring the
iterator type and the consuming method:

```
type DataStream:
  pending: Vec[Event]
  cursor: isize

fulfill IntoIterable for DataStream:
  type Iter = DataStreamIntoIter
  fn consuming_iterator(s: DataStream) -> DataStreamIntoIter:
    DataStreamIntoIter(...)    // takes ownership of s's pending events
```

The `for x in d` syntax (with `d: DataStream`) dispatches to this
implementation automatically, consuming `d`.

#### 12.9.5 Both `Iterable` and `IntoIterable` for the same type

Stdlib types typically implement both `Iterable` (borrowing iteration)
and `IntoIterable` (consuming iteration). The user picks at the call
site:

- `for x in v:` — `IntoIterable` dispatch; consumes v.
- `for x in &v:` — `Iterable` dispatch; borrows v.

A user-defined type may implement one, both, or neither. If a type
implements only `Iterable`, the consume form `for x in v` is a compile
error (no `IntoIterable` impl); the user must use `&v`. If it
implements only `IntoIterable`, the borrow form `for x in &v` is a
compile error.

There is no "reclaim after consumption." Once `consuming_iterator(v)`
is called, `v`'s binding is consumed and the source's elements are
either yielded (now owned by the body's bindings) or remaining in the
iterator (to be dropped when the iterator is dropped). If the user
needs the source after iteration, they choose the borrow form, or they
restructure to consume-and-rebuild (pass the source through a
transformation function that consumes and returns a new collection).

### 12.10 Built-in Iteration Sources

Stdlib provides both `Iterable` and `IntoIterable` implementations for
the language's built-in iterable types:

- **Ranges (`Range[T]`)** — `Range[T]` is `Copy`. Both forms work; from
  the user's perspective, `for i in 0..N:` and `for i in &(0..N):` are
  indistinguishable. The conventional form is the consume form.
- **Arrays (`T[N]`)** — implement both `Iterable` (borrow) and
  `IntoIterable` (consume). See §12.10.1 for details.
- **Stdlib collections** (`Vec[T]`, `HashMap[K, V]`, etc.) — implement
  both, with iterator types specific to each container. The specific
  Item types and yielding semantics are stdlib design decisions.

These implementations are language-privileged per §3.7.3 and are not
user-overridable.

#### 12.10.1 Iterating over arrays

Arrays implement both forms. The user picks at the call site:

**Consume form** (`for x in arr:`): the array is consumed. Each
iteration variable is owned `T`. After the loop, `arr` is no longer
usable.

For `T: Copy` (e.g., `f32[1024]`), the Copy element behavior is
identical to borrow form — `sample` is a Copy value either way:

```
let buf: f32[64] = make_block()
mut sum: f32 = 0.0
for sample in buf:                // sample: f32; buf consumed
  sum = sum + sample
// buf cannot be used after this loop
```

For non-`Copy` `T`, each iteration moves one element out of the array's
storage. The body has full ownership:

```
mut destinations = Vec::new()
let records: Record[16] = make_records()
for r in records:                  // r: Record (owned); records consumed
  destinations = destinations.push(r)
// records cannot be used; destinations holds all the records
```

**Borrow form** (`for x in &arr:`): the array is borrowed for the
duration of the loop. Each iteration variable is either `T` (for Copy
elements) or `&T` (for non-Copy elements). After the loop, the array
remains owned.

```
let buf: f32[64] = make_block()
mut sum: f32 = 0.0
for sample in &buf:               // sample: f32; buf borrowed
  sum = sum + sample
process(buf)                       // ✓ buf still owned

let records: Record[16] = make_records()
for r in &records:                 // r: &Record
  print(r.first_name)              // ✓ read access only
  consume(r)                       // ✗ cannot move out of borrow
process(records)                   // ✓ records still owned
```

While the array is borrowed (during the for-loop expression), indexed
writes (`arr[i] = v`) are forbidden per §11.9.2. Indexed reads
(`arr[i]`) are allowed (reading is non-disruptive).

#### 12.10.2 Iterating over ranges

`Range[T]` iteration is the basic counting pattern:

```
for i in 0..N:
  process(i)
```

`Range[T]` is `Copy` (for `T: Copy`, which all built-in integer types
satisfy). The consume form `for i in 0..N:` consumes a Copy value,
which is functionally indistinguishable from borrowing — the source
expression is a literal anyway, not a binding the user would want to
reuse. The borrow form `for i in &(0..N):` is also legal but rarely
used.

Ranges and their iterators are stack-allocated; iteration has no heap
overhead.

### 12.11 Iteration Performance

The combination of (1) the linear-ownership optimization for the
`Iterator::next` tuple-return pattern (§12.7.2), (2) monomorphization of
generic iterator implementations (§2.3), and (3) inlining of small
iterator methods produces machine code equivalent to hand-written
indexed loops.

For a typical numeric inner loop (DSP block processing, where the
buffer is needed after the loop):

```
mut sum: f32 = 0.0
for sample in &audio_block:
  sum = sum + sample * sample
// audio_block still owned; available for further processing
```

A conforming implementation compiles this to machine code with no heap
allocation, no iterator object lifecycle overhead, and no per-iteration
function call cost. The iterator's cursor is held in registers; the
`next` call is inlined; the loop is a tight machine loop over the
array's elements.

This performance behavior is a *required* property of conforming
implementations. The trait-based source-level abstraction is intended to
disappear at the machine level for monomorphized loops over built-in
iterables.

### 12.12 Interaction with the Rest of the Language

#### 12.12.1 Pattern matching and iteration variables

The iteration variable may be a pattern, not just a single name. The
pattern destructures each yielded value:

```
let pairs: Vec[(i32, string)] = ...
for (id, name) in pairs:
  process(id, name)
```

The pattern follows the rules of §6.2.4 and §9.2.2. Refutable patterns
(those that may fail to match) are not permitted as iteration variables;
the iteration variable must always bind successfully for each yielded
value. To filter, the body uses `continue`:

```
for x in items:
  match x:
    Special(payload): continue        // skip; for filtering, use continue
    Other(data): process(data)
```

A future extension may add `for pattern if guard in iterable:` syntax for
inline filtering; not in v1.

#### 12.12.2 Loops in trait method bodies

Trait method bodies may contain loops, subject to the standard mutation
and ownership rules. Default-body methods in trait declarations (§3.1.3)
may use loops:

```
trait Statistics:
  fn samples(value: &Self) -> Vec[f32]

  fn count_above(value: &Self, threshold: f32) -> isize:
    mut count: isize = 0
    let elements = samples(value)
    for s in elements:                   // consumes the returned Vec; s: f32 (Copy)
      if s > threshold:
        count = count + 1
    count
```

The default body's loop is part of the trait declaration; implementations
may override it as usual. The `samples` method here is abstract (no
default body); each implementation provides its own. The `count_above`
default body iterates the returned `Vec[f32]` to compute the result.

#### 12.12.3 Loops in generic function bodies

A generic function body containing a loop is type-checked at definition
per §2.2.2. The loop's iterable expression's type must satisfy
`Iterable` (for borrow form) or `IntoIterable` (for consume form) at
the call site for each monomorphization. Associated-type constraints
use `.` member-access notation per §3.1.2:

```
fn total[T: Iterable](source: &T) -> T.Iter.Item where T.Iter.Item: Numeric:
  mut sum = T.Iter.Item::zero()
  for sample in source:
    sum = sum + sample
  sum
```

The compiler verifies the constraints at definition and monomorphizes
per call site.

### 12.13 Interaction with Reactivity (Forward-Looking)

The reactive system is specified in §13. Loops in reactive contexts
follow §11.14:

- A `derived` expression's body may contain loops. Each evaluation of
  the derived expression runs the loop fresh. The loop's local
  mutations are not observable outside the derived's evaluation; only
  the derived's final value enters the reactive graph.
- The collection or range being iterated in a derived body may itself
  be a reactive value. Each time the reactive value updates, the
  derived re-evaluates and the loop re-runs.
- The `while` loop's condition may depend on reactive values, but
  reactive updates do not interrupt an in-progress loop iteration.

Full specification is given in §13 (see §13.10 for the evaluation
cycle and §13.12 for the reactivity boundary).

### 12.14 Restrictions and Edge Cases

#### 12.14.1 Empty iteration

An empty iterable (such as `0..0` or an empty container) produces no
iterations. The loop's body does not execute. The expression value (in
expression context) is determined by the else-clause-and-break-value
table of §12.6.2:

```
let result = for x in 0..0:         // empty range (type-resolvable)
  break x
else:
  default_value
                                    // result = default_value
```

#### 12.14.2 Iterators that never complete

An iterator whose `next` always returns `Some(_)` produces an infinite
loop. There is no language-level prevention; the responsibility lies with
the iterator implementation. A `break` inside the body is the user's
mechanism for terminating such loops.

#### 12.14.3 Side effects in iterator implementations

`Iterator::next` is a pure function in the type-system sense: it takes
inputs and produces outputs. However, the iterator's value contains
state that the function may mutate (via `mut local`-style rebind in the
body). Different invocations of `next` produce different results because
the cursor advances; this is the normal behavior of iteration and does
not violate purity.

Iterators must not perform externally observable I/O. The reactive
system's signals (§13) are the appropriate mechanism for
externally-driven sequences; iterators are for collection traversal.

---

*End of §12.*

---

## 13. Reactive System

This section specifies the language's reactive composition layer: the
declaration kinds (`signal`, `attr`, `recurrent`, `derived`), the
composition constructs (`node`, `connection`, parts), the rules
governing reactive expression evaluation, and the host API through
which external code drives and observes the reactive graph.

The reactive system is the language's mechanism for expressing values
that change over time. Ordinary computation in Ductus is pure and
immutable (§1.3, §11.1); change is confined to two contexts: local
mutation within a function body (§11) and the reactive system
specified here. The reactive system gives users a declarative way to
express "this value depends on these other values, and recomputes
when they change" without manually wiring update propagation.

### 13.1 Design Principles

The reactive system is built on seven load-bearing principles.

**Declarative composition.** A reactive graph is built declaratively
from signal, attr, recurrent, derived, node, and connection
declarations. Placement syntax (§13.8) constructs instances.
Composition is structural — the graph's shape is known at compile
time.

**Static graph.** Once constructed, the reactive graph's structure
is fixed for the lifetime of the kernel instance. Signals, attrs,
recurrents, nodes, and connections are created at startup and not
added or removed at runtime — except by hot reload (§13.15), which
replaces the program source and applies a diff atomically.

**Pure evaluation surface.** Reactive expressions (`derived`
declarations, attr default expressions, recurrent arm expressions)
are pure expressions over signal, attr, recurrent, and derived
values. They contain no `mut` bindings, no loops, no
statement-level imperative constructs. When imperative work is
needed, the reactive expression calls a pure function (per §11),
which may use `mut` internally.

**Lazy, batched evaluation.** Writes (signal, attr) mark dependent
cells dirty without immediate recomputation. The kernel evaluates
the dirty set in topological order, advances recurrent cells per
their firing arm expressions in lockstep, and swaps the back
buffer atomically — all in a single `kernel.publish()` operation
(§13.14.4). Writes accumulate between publishes; one publish
processes the union.

**Cycles handled at two layers.** Reactive expression cycles are
handled at the cell level: derived↔derived cycles are forbidden
(no temporal break possible); recurrent self-reference and
cross-reference are allowed because lockstep treats recurrent
reads as previous-committed values. Topology cycles between nodes
via connections are handled separately via the `Circularity` trait
(§13.6, §13.11): a topology cycle is valid only if it traverses at
least one connection type satisfying `Circularity`.

**Reactive composition uses nodes, parts, and connections.**
Reactive cells (signal, attr, recurrent, derived) may hold values
of any type; the kernel chooses a storage strategy from §13.12.4:
direct in-cell storage for values fitting the platform atomic
word, or handle-based pool storage for larger or dynamically-sized
values. Imperative data structures (`Vec`, `HashMap`, etc.) are
first-class as values held inside reactive cells via pool storage,
and are also usable as ordinary owned values inside function
bodies (governed by §11). The reactive system organizes
composition and propagation; cell content is governed by ordinary
type rules.

**Separation of topology and outside-world alignment.** The reactive
system carries the following construct kinds, each with a focused
role:

- `fn` (§11) — pure compute. Reactive-transparent.
- `operator` (§13.17) — stateful reactive transform from cells to
  cells. Pure with respect to outside reality.
- `node` and `connection` (§13.3, §13.6) — topology. Composable
  graph structure traversed by the kernel via `expose:` (§13.3.7).
- `stream` (§13.18) — append-only reactive primitive for event-
  shaped flows.
- `effect` (§13.19) — outside-world alignment via the
  reconciliation model. The host-interpreted bridge between program
  state and the runtime environment.

Earlier drafts of this specification collapsed outside-world effects
into nodes the host interpreted: a `Log` node with a `message` attr,
a `Delay` node with a `time` attr, and so on. The host walked the
node graph and dispatched on type. The motivating argument was
parsimony — one composition layer (nodes + parts + connections +
reactive cells) covering both data flow and effects.

That design conflated two distinct concerns. Topology — the node-
and-connection graph the kernel traverses — has its own
discipline: structural identity, child placement, connection
endpoints, exposition. Outside-world alignment — sending a request,
opening a connection, playing audio — has a different shape:
request/response, long-lived resources, event streams, bidirectional
flows, lifecycle entangled with program state. Forcing both into a
single mechanism either flattened topology's structural advantages
or distorted effects' alignment semantics.

The present design separates them. Effects are first-class via the
`effect` construct (§13.19), using a reconciliation model
(`desired:` / `observed:`) that subsumes request/response, long-
lived resources, fire-and-forget, and event-stream shapes under one
primitive. Nodes remain topology; runtime hosts are free to
interpret nodes of specific types as before (DSP node graphs,
UI children, music clips), but are not obliged to use nodes for
outside-world alignment.

The trade-off has moved. Effects carry interpretation complexity in
the host's reconciler implementations (which is the right place — the
host knows its domain). The language carries two distinct construct
kinds (nodes for topology, effects for alignment) rather than one
overloaded kind. The conceptual clarity and the alignment with
standard reconciliation patterns (Kubernetes controllers, Erlang/OTP
supervision, Elm subscriptions) outweighs the additional construct.

Adding a new effect type is achieved by declaring a new `effect`;
the host registers a reconciler for that effect type. Adding a new
topological participant is achieved by declaring a new `node`; the
host extends its interpreter for traversed node types (e.g., DSP
graph evaluation). The two paths are no longer entangled.

#### 13.1.1 A small example

A complete reactive program that counts ticks of a host-driven
signal and exposes the count through a connection to a Display.
The signal named `tick` in this example is *user-defined* — it is
not a language built-in. Ductus has no built-in clock or tick
primitive; hosts that need a clock declare their own signal and
write to it at whatever cadence is meaningful for their domain.

```
-- Module-level signal; user-defined, not a language built-in.
-- The host writes to this signal at its own cadence.
signal tick: i64 = 0

-- Counter advances its count whenever the host changes `tick`.
node Counter:
  recurrent count: i32 = 0
    | on tick: self.count + 1
  out: ShowsCount [=1]

-- Display reads the count through its incoming connection.
node Display:
  attr label: string = "Unnamed"
  in: ShowsCount [=1]
  derived shown: string = "{self.label}: {self.in.ShowsCount[0].count}"

-- Connection from Counter to Display carries a derived count.
connection ShowsCount:
  from: Counter
  to: Display
  derived count: i32 = self.from.count

-- Placements (instances).
Counter c1:
  ShowsCount: d1                -- outgoing connection to d1

Display d1 | label="Main"
```

The host drives the program via:

```
loop {
  kernel.write_signal(tick_id, next_tick_value);   // accumulate dirty bits
  kernel.publish();                                 // evaluate + atomic swap
  // consumers observe d1.shown via kernel.swap()
}
```

Each `publish()`:

1. Detects that `tick` differs from its previous-published value
   (dirty).
2. Re-evaluates `Counter.count`'s arm (its trigger `tick` fired).
3. Re-evaluates `ShowsCount.count` and `Display.shown` (transitive
   derived dependencies).
4. Commits recurrent advancement and atomically swaps the back
   buffer for consumer visibility.

This example demonstrates every reactive declaration kind (signal,
attr, recurrent, derived), composition through nodes and connections,
cardinality (`[=1]`), placement with overrides, indexed access
through the connection (`self.in.ShowsCount[0].count`), and the
publish-driven evaluation cycle.

### 13.2 Reactive Declarations

The reactive system has six declaration kinds, distinguished by
who controls the value and how (or whether) it changes over time.
Four declare value-shaped reactive cells (signal, attr, recurrent,
derived); one declares an event-sequence-shaped reactive cell
(stream, full treatment in §13.18); one is a compile-time constant
(const).

#### 13.2.1 `signal`

```
signal name: Type = initial
```

A `signal` declares a writable reactive cell. The initial value is
supplied at the declaration. After construction, the value is written
only through the host API (§13.14.2); Ductus source has no
syntactic form for assigning to a signal.

Signals represent reactive *entry points* — values fed into the
reactive graph by the host or runtime, not computed by Ductus
source. The host pushes new values into the kernel; the reactive
graph propagates the changes.

Signals may be declared at three scopes:

**Module-level signals** — declared at module top level (outside
any node or connection body). One value shared across the program;
host writes it; all references read the same cell. Useful for
program-wide inputs: a global clock, a user-input axis, a master
volume signal.

```
signal mouse_x: i32 = 0
signal current_time_ms: i64 = 0
signal master_volume: f32 = 0.5
```

**Node-level signals** — declared inside a node body. Per-instance:
each placement of the node creates its own cell. The runtime/host
writes per-instance signals to feed instance-specific data into the
graph (an HTTP response for a specific `Fetch` instance, a sensor
reading for a specific `Sensor` instance, etc.).

```
node Fetch:
  default attr url: string
  signal response: Result[HttpResponse, HttpError] = Err(NotYetFetched)
  signal status: i32 = 0
```

(Types like `HttpResponse`, `HttpError`, and variants such as
`NotYetFetched` are illustrative; the stdlib or a host package
provides concrete definitions.)

**Connection-level signals** — declared inside a connection body.
Per-instance per-connection: each placement of the connection
creates its own cell. The runtime writes per-connection signals to
feed data flowing through that specific connection instance (bytes
received on a network connection, audio samples through a routing
edge, etc.).

```
connection NetworkChannel:
  from: Source
  to: Sink
  signal bytes_received: Bytes = empty_bytes
  signal status: ChannelStatus = ChannelStatus::Idle
```

(Types like `Bytes`, `ChannelStatus`, `Source`, and `Sink` are
illustrative; the stdlib or domain code provides concrete
definitions.)

In all three scopes, signals share the same semantics: host-written,
not source-assignable, reactive (writes trigger downstream
re-evaluation). The scope determines instance multiplicity and how
the host addresses the signal when writing (§13.14.2).

Use cases by scope:

- Module-level: program-wide entry points (one cell, shared).
- Node-level: per-node-instance runtime-fed data.
- Connection-level: per-connection-instance runtime-fed data.

Per-instance *configuration* (user-controlled) is the role of
`attr` (§13.2.2); per-instance memory is the role of `recurrent`
(§13.2.4). Signals are reserved for externally-fed reactive inputs.

#### 13.2.2 `attr`

```
attr name: Type = default       // with default — placement may override
attr name: Type                 // no default — placement must supply
```

An `attr` declares a writable reactive cell that is *per-instance* of
its enclosing node or connection type. Each instance carries its own
cell. Like signals, attrs are written only through the host API or
at placement time (§13.8).

An attr declaration may include a `= default` initializer or omit it:

- **With default** (`attr name: Type = default`): the attr has a
  fallback value at construction. Placement may override the default
  with an explicit value but is not required to.
- **Without default** (`attr name: Type`): the attr has no fallback.
  Every placement of the enclosing type must supply a value for this
  attr (via the attribute clause, a flag, or the `/expr` slot if the
  attr is also the type's `default attr`). Omitting it at placement
  is a compile error. This is the *required-at-placement* form.

The required form is used when no sensible default exists — an
identifier the user must choose, an external resource handle, an
endpoint URL. Surfacing the requirement in the type signature is
preferable to picking an arbitrary default that masks misuse.

```
node Driver:
  attr expertise_level: i32 = 5
  attr risk_tolerance: f32 = 0.5
  attr is_active: bool = true

node Synthesizer:
  attr master_volume: f32 = 1.0
  attr current_pitch: f32 = 440.0

node Endpoint:
  attr url: string                    // no default — every placement must set
  attr method: string = "GET"         // has default — placement may omit
```

The `default` expression, when present, provides the initial value
used when an instance is constructed without an explicit value for
that attr. Defaults may reference previously-declared attrs of the
same node (declaration order is significant; see §13.2.6).

The default may be a constant expression, an expression involving
other declared attrs, an expression involving signals visible in
scope, or any compile-time-evaluable expression.

```
node Filter:
  attr cutoff_hz: f32 = 1000.0
  attr resonance: f32 = self.cutoff_hz / 1000.0      // references earlier attr
  attr enabled: bool = true
```

At placement time, the user may override the default by supplying a
value (§13.8.2):

```
Filter f1 | cutoff_hz=500.0          // override default; resonance and enabled use defaults
```

For attrs without defaults, the placement value is mandatory; the
attr's cell receives that value at construction and is reactive from
that point on, exactly as if the value were a default.

##### 13.2.2.1 `default attr`

A node or connection type may designate one of its attrs as the
*positional default* by prefixing the declaration with `default`:

```
node Log:
  default attr message: string

connection Drives:
  from: Driver
  to: Drivable
  default attr aggressiveness: f32 = 0.5
```

A `default attr` is a regular attr in every respect (writable,
overridable at placement, can have a default value) plus one
property: it becomes the target of the positional `/expr` syntax at
placement time (§13.8.5), so the attr can be set without naming it.

The positional and named forms are equivalent and interchangeable —
the default attr remains settable by name via the attribute clause
(§13.8.7) or by flag (§13.8.8), exactly like any other attr; `/expr`
is an additional, optional positional shortcut:

```
// Node:
Log /"Hello World"                          // /expr sets default attr `message`
Log | message="Hello World"                 // equivalent attribute-clause form

// Connection (assuming Drives declares `default attr aggressiveness: f32 = 0.5`):
Drives/0.8: some_car                        // /expr sets default attr `aggressiveness`
Drives | aggressiveness=0.8: some_car       // equivalent attribute-clause form
```

Rules:

- At most one `default attr` per type. Declaring two is a compile
  error.
- The `default attr` marker applies only to `attr` declarations.
  `recurrent`, `derived`, `const`, and `signal` cannot be marked
  `default`.
- The mechanism is uniform across nodes and connections: at placement
  time, `/expr` binds the type's default attr regardless of whether
  the placed type is a node (§13.8.5.2) or a connection (§13.8.5.1).
  Connections supply their destination separately, in the placement's
  body (§13.8.5.1); the destination is not an attr and is not
  targeted by `/expr`.

#### 13.2.3 `derived`

```
derived name: Type = expression
```

A `derived` declares a *read-only* reactive value defined by an
expression. The kernel maintains the value consistent with its
inputs: when any signal, attr, recurrent, or other derived that
the expression reads changes, the expression re-evaluates (under
the lazy-batched rules of §13.10).

```
node Driver:
  attr expertise_level: i32 = 5
  attr risk_tolerance: f32 = 0.5
  derived skill_factor: f32 = self.expertise_level as f32 / 10.0
  derived is_aggressive: bool = self.risk_tolerance > 0.7
```

A derived's expression is a *pure expression* — no `mut`, no loops,
no statements. It may include:

- Arithmetic and comparison operations on reactive and non-reactive
  values.
- Reads of signals, attrs, recurrents, and other deriveds (these
  create reactive dependencies).
- Field accesses and indexed reads.
- Function calls (functions are reactive-transparent; §13.12.2).
- Pattern matching (`match` expressions).
- Conditional expressions (`if`/`else`).
- Closure construction (the closure captures values at construction
  time; §13.12.3).

The expression's *provenance* — the set of reactive cells it reads,
including transitively through function calls — determines its
dependency set. When any cell in the dependency set changes, the
derived becomes dirty and is recomputed at the next publish.

##### 13.2.3.1 Scope

A `derived` may be declared at three scopes, paralleling `signal`:

- **Module-level** — declared at module top level (outside any node
  or connection body). One cell shared across the program. References
  to module-level deriveds use the bare name (no `self.` prefix).
- **Node-level** — declared inside a node body. Per-instance: each
  placement of the node creates its own cell.
- **Connection-level** — declared inside a connection body.
  Per-instance per-connection: each placement of the connection
  creates its own cell.

Module-level deriveds are useful for global computed values that
many parts of the program depend on (a normalized clock, a derived
configuration value, etc.). Their initial values must reference
only cells visible at module scope per the topological-init rule
(§13.2.6).

#### 13.2.4 `recurrent`

```
recurrent name: Type = initial
  | on triggers: next_expr
  | on triggers where guard: next_expr
  ...
```

A `recurrent` declares a *per-instance* writable cell with memory
across triggering events. It is the mechanism for values that
depend on their own past — counters, accumulators, smoothing
curves, running statistics, sequencer step indices, and other
patterns where the new value depends on the previous value.

A recurrent declaration consists of:

- **`= initial`** — the value the cell holds before any arm fires.
  Mandatory.
- **One or more arms** — each beginning with `| on`, specifying the
  trigger cells whose value changes cause the arm to fire, an
  optional `where guard` clause, and the `next_expr` to apply when
  the arm fires. At least one arm is required; a recurrent with no
  arms is a compile error.

An arm has the shape:

```
| on triggers: next_expr
| on triggers where guard: next_expr
```

where:

- **`triggers`** is one or more reactive cells (signals, attrs,
  recurrents, deriveds) whose value changes cause the arm to fire.
  A single trigger is written as a bare name (`on tick`); a group
  of two or more triggers is written parenthesized
  (`on (tick1, tick2)`). Parens are optional for a single trigger
  and required for groups. Trigger semantics is value-change
  (§13.2.4.4).
- **`where guard`** is an optional reactive boolean expression
  evaluated in the recurrent's scope (§13.2.4.7), with the same
  purity rules as derived expressions (§13.2.3). When present, the
  arm fires only if at least one trigger changed value AND the
  guard is currently true.
- **`next_expr`** is a pure reactive expression. When the arm
  fires, the expression evaluates against current inputs and the
  cell's *previous-committed* value; the result becomes the cell's
  new value after the evaluation pass commits.

```
signal increment: i32 = 0

node Counter:
  attr step_size: i32 = 1
  recurrent count: i32 = 0
    | on increment: self.count + self.step_size
```

The cell starts at 0. When `increment`'s value changes (host
writes a new value via `write_signal`), the arm fires: the
`next_expr` reads `self.count` (previous-committed value) and
`self.step_size`, computes the new value, and commits at the end
of the evaluation pass.

**Priority on overlapping fires.** When multiple arms' triggers
fire in the same publish, arms are evaluated in declaration order;
the first arm whose trigger fired AND whose `where` guard (if
present) is true wins. The remaining arms are not evaluated.
Earlier-declared arms have higher priority. If no arm fires (no
triggers changed, or all guards are false), the recurrent holds
its previous value.

##### 13.2.4.1 Lockstep advancement

When multiple recurrent cells' triggers fire in the same
evaluation pass (a single `kernel.publish()` cycle), they advance
in **lockstep**: every triggered recurrent's firing arm expression
reads the *previous-committed* values of all recurrent cells in
the system (including other triggered ones), computes a new value,
and commits together at the end of the pass. No recurrent cell
sees another recurrent cell's just-advanced value within the same
pass.

Recurrents whose triggers did not fire in this pass do not
re-evaluate; they retain their existing values.

This is the standard synchronous-dataflow semantics (Lustre,
Esterel, Verilog `<=` non-blocking assignment). The new value of
any triggered recurrent is a pure function of the previous-
committed values and the inputs received during this pass.

##### 13.2.4.2 Recurrent vs attr

`attr` and `recurrent` are both per-instance writable cells. The
distinction is who advances the value:

- `attr` cells change only when the host writes via
  `kernel.write_attr`. The kernel does not advance them
  automatically.
- `recurrent` cells advance automatically per their firing arm's
  expression whenever a trigger in an arm fires. The host cannot
  directly write a recurrent cell at runtime; control is indirect
  — the host triggers signals (or writes attrs) that an arm's
  expression reads or that appear in an arm's trigger list.

Use `attr` for parameters, configuration, and host-controlled
inputs. Use `recurrent` for cells that carry computed values
across triggers.

##### 13.2.4.3 Override at placement

The `= initial` value may be overridden at placement, similar to
attrs:

```
Counter c1 | count=100      // override initial value
```

The arm structure (triggers, guards, and `next_expr` expressions)
is a structural type property and *cannot* be overridden at
placement. If per-instance variation is needed, parametrize via
attrs read inside `next_expr`:

```
signal tick_signal: u64 = 0

node Counter:
  attr step_size: i32 = 1
  recurrent count: i32 = 0
    | on tick_signal: self.count + self.step_size

Counter c1 | step_size=5    // per-instance step via attr override
```

##### 13.2.4.4 Value-change semantics for triggers

A trigger in an arm's trigger list fires when the listed cell's
value changes from the perspective of the evaluation pass. Writing
the same value to a signal does not fire its triggers. This is
standard reactive semantics — only meaningful changes propagate.

To express "fire on every event regardless of value," use a
counter pattern: the signal is a monotonically increasing count;
each "event" increments the count; downstream cells trigger on
every increment because the value changes each time.

##### 13.2.4.5 Scope

A `recurrent` may be declared at three scopes, paralleling `signal`
and `derived`:

- **Module-level** — declared at module top level. One cell shared
  across the program. References use the bare name (no `self.`
  prefix). Useful for global stateful counters, accumulators, or
  state machines driven by module-level signals.
- **Node-level** — declared inside a node body. Per-instance.
- **Connection-level** — declared inside a connection body.
  Per-instance per-connection.

##### 13.2.4.6 Tuple-coupled recurrents

Multiple recurrents may share a single arm evaluation by declaring
them as a tuple:

```
recurrent (name1, name2, ...): (Type1, Type2, ...) = (init1, init2, ...)
  | on triggers: tuple_expression_returning_same_shape
```

The declaration creates N independent cells, each named and
typed individually. The arm's `next_expr` returns a tuple of the
same shape and types; all N cells advance atomically from a single
evaluation. Shared computation in the arm body is performed once,
not N times.

Example — a Kalman filter sharing the gain computation across
mean and variance updates. The arm body is a single pure
expression and cannot directly contain `let` bindings; the shared
work is factored into a helper function whose body computes the
gain once and returns the pair of updated values:

```
signal source: f32 = 0.0
signal noise: f32 = 0.01

fn kalman_step(mean: f32, variance: f32, source: f32, noise: f32) -> (f32, f32):
  let gain = variance / (variance + noise)        // computed once per call
  (
    mean + gain * (source - mean),                // updated mean
    (1.0 - gain) * variance,                      // updated variance
  )

recurrent (mean, variance): (f32, f32) = (source, 1.0)
  | on source: kalman_step(mean, variance, source, noise)
```

The single function call evaluates the shared `gain` once per
publish and returns both updated values atomically. Without
tuple-coupled recurrents, the same logic would require two
independent recurrents each calling the helper separately, doing
the gain computation twice per publish.

Reads of any cell within the tuple use its individual name
(`self.mean`, `self.variance`, or bare `mean`/`variance` at
module scope).

Lockstep semantics (§13.2.4.1) are preserved across the tuple:
during arm evaluation, reads of any cell in the group return
its previous-committed value. Cross-references within the tuple
read previous-committed values, the same way independent
recurrents do.

In the per-publish DAG (§13.11.3), tuple-coupled recurrents
contribute one evaluation node with N output edges, not N
independent evaluation nodes.

Initial values follow the topological-init rule (§13.2.6) — each
cell's initial may reference any reactive cell in scope, provided
no init-time cycle exists. If any one cell's initial creates a
cycle, the entire group is rejected.

##### 13.2.4.7 Conditional triggers (`where` clauses)

An arm may carry a `where guard` clause placed after its trigger
list and before the colon:

```
recurrent name: Type = init
  | on trigger_cell where guard: expr
```

The arm fires when `trigger_cell` changes value AND `guard` is
currently true. A change in `guard` alone (without `trigger_cell`
changing) does not fire the arm.

Each arm carries its own optional `where` clause; different arms
may apply different guards:

```
recurrent name: Type = init
  | on A where p1: next_A_expr
  | on B where p2: next_B_expr
  | on C: next_C_expr
```

Reads as: "fire on A-change if p1; or on B-change if p2; or on
any C-change," evaluated in declaration order per the priority
rule in §13.2.4.

The guard is a reactive boolean expression evaluated in the
recurrent's scope, with the same purity rules as derived
expressions (§13.2.3).

Pedagogically, an arm guard is equivalent to inlining the
guard into the arm's `next_expr`:

```
// guard form
recurrent x: T = init
  | on trigger_cell where guard: expr

// inline-conditional form, observationally identical
recurrent x: T = init
  | on trigger_cell: if guard then expr else x
```

The two produce identical observable behavior, but the `where`
form allows the kernel to skip the arm's evaluation entirely when
the guard is false. This is a perf benefit when the `next_expr`
is expensive and the guard is cheap, and it allows the priority
rule to fall through to subsequent arms when the guard fails.

Example shown at module scope (cell references are bare; inside a
node body, references would use `self.counter` etc.):

```
signal reset_signal: bool = false
signal tick: u64 = 0
signal running: bool = true

recurrent counter: i32 = 0
  | on reset_signal: 0                         // arm 1: reset to zero
  | on tick where running: counter + 1         // arm 2: increment if running
```

If both `reset_signal` and `tick` change in the same publish, arm
1 wins per the priority rule (§13.2.4) and `counter` becomes 0
(not `counter + 1`).

##### 13.2.4.8 Dynamic-size cell types

Recurrent cells may hold dynamic-size types in addition to
fixed-size types. Dynamic-size types include:

- `Vec[T]` — persistent vector with structural sharing
- `SmallVec[T; N]` — inline up to N elements, then heap
- `RingBuf[T; N]` — fixed-capacity ring buffer

Storage and cost details are specified in §13.12.4 (cell types
and storage). An arm's `next_expr` returns a new value of the
declared type; the kernel handles allocation and triple-buffer
rotation transparently. Source code never mutates a cell in
place — the functional builder API (`.with(value)`, `+`
operator) returns new collection values.

#### 13.2.5 `const`

```
const name: Type = value
```

A `const` declares a compile-time constant value associated with a
node or connection type. Unlike `attr`, `recurrent`, and `derived`,
a `const` is not reactive and not per-instance in the runtime
sense — it is a type-level property whose value is the same for
every instance of the type and is fixed at compile time.

```
trait Action

node Log:
  satisfies Action
  const type: string = "@action/log"
  default attr message: string

node Delay:
  satisfies Action
  const type: string = "@action/delay"
  default attr time: duration
```

##### 13.2.5.1 Properties

- **Compile-time value.** The right-hand side must be evaluable at
  compile time. It may reference other consts (of the same type or
  top-level), literal values, and any compile-time-evaluable
  expression. It may not reference reactive cells (signals, attrs,
  recurrents, deriveds), since those are runtime values.
- **Not reactive.** A const value never changes during the kernel's
  lifetime. It does not occupy a cell in the reactive state buffer
  and does not participate in dirty propagation.
- **Allowed complex types.** Because consts are not stored in the
  single-cell reactive buffer (§13.12.4), they may hold complex
  values: records, arrays, tuples, nested structures. The
  single-cell constraint does not apply.
- **Not overridable at placement.** A const's value is fixed by the
  declaration; placement bodies cannot override it. Attempting to
  set a const at placement is a compile error.
- **Not host-writable.** The host API has no `write_const`. Consts
  are immutable for the kernel's lifetime.

##### 13.2.5.2 Access forms

A const is accessible through three syntactic forms:

- **Instance-level (`self.<const>`)** — inside the declaring node
  or connection's reactive expressions. Resolves to the same value
  as the type-level access.
- **Through an instance (`<instance>.<const>`)** — from function
  bodies or other instances' bodies that hold a reference to an
  instance of the type.
- **Type-level (`<TypeName>::<const>`)** — direct type-level access
  without an instance. Useful for compile-time discriminators and
  dispatch tables.

```
-- Type-level access lets callers read a type's const without an
-- instance. Useful for compile-time tables and dispatch keys.
const ACTION_LOG_TAG: string = Log::type        -- "@action/log"
const ACTION_DELAY_TAG: string = Delay::type    -- "@action/delay"

fn tag_for[T: Action](_: T) -> string:
  T::type           -- type-level read; no instance needed at runtime
```

##### 13.2.5.3 Declaration order

Within a node or connection body, a const's value expression may
reference previously-declared consts of the same body (in
declaration order). Referencing a later-declared const is a
compile error.

#### 13.2.6 Initial value rules

Initialization happens in two phases: compile-time resolution
(consts) and a startup pass (reactive cells). Within the startup
pass, all reactive cells are initialized in **topological order
over their init-time read dependencies** — there are no separate
serialized steps for signals, attrs, recurrents, and deriveds.

**Compile-time resolution (during compilation):**

1. **Top-level consts** are resolved in declaration order. Values
   are embedded in the compiled artifact; no runtime computation
   is needed.
2. **Per-type consts** declared inside node/connection bodies are
   similarly resolved at compile time. They are not allocated
   cells in the reactive state buffer.

**Startup pass (during kernel initialization):**

The kernel constructs an *init-time dependency graph*: each
reactive cell (signal, attr, recurrent, derived) is a node;
edges run from each cell to the cells its initial-value
expression reads. The kernel then evaluates initial values in
topological order over this graph.

For each cell:

- **Signals** evaluate their `= initial` expression.
- **Attrs** are initialized when their containing instance is
  placed. For each attr:
    - If the placement supplies an explicit value (via the
      attribute clause, flag, or `/expr` for the default attr),
      that value is evaluated and stored.
    - Otherwise, if the attr was declared with `= default`, the
      default expression is evaluated.
    - Otherwise, the attr was declared without a default and the
      placement omitted a value — a compile error caught before
      startup (see §13.2.2).
- **Recurrents** evaluate their `= initial` expression. The arm
  expressions are *not* evaluated at startup; recurrent cells hold
  their initial values until a trigger fires. When an initial-value
  expression reads a `Signal[T]` cell (per §13.2.8), the read
  returns the cell's value at the topological-init evaluation
  point — equivalent to a snapshot of the cell at startup. The
  recurrent does not subscribe to subsequent changes of that cell;
  for tracking semantics use a `derived` declaration instead. The
  same snapshot semantic applies to attrs and signals whose initial
  expressions read other reactive cells.
- **Deriveds** evaluate their expression body.
- **`when` predicates** (§13.9) are evaluated alongside deriveds
  in the topological order. Each instance's initial gate state is
  established here. An instance whose `when` evaluates to false at
  the end of startup begins inactive, with its other cells holding
  their just-computed initial values per Model B (§13.9.7).

**Init-time dependency rules:**

- An initial-value expression may read any reactive cell visible
  in scope, regardless of declaration kind. The topological sort
  resolves ordering automatically. There is no artificial
  "recurrents init before deriveds" constraint.
- **Init-time cycles are compile errors.** A cycle in the init
  dependency graph (cell A's initial reads B; B's initial reads
  A; or longer cycles) cannot be resolved by topological sort.
  This is distinct from runtime cycles (§13.11), which the
  per-publish DAG handles via recurrents-as-delays. Init time
  has no notion of "previous publish," so cycles flat-out fail.
- Within a node body, an attr or recurrent's initial may
  reference previously-declared cells of the same body. The
  topological sort catches forward references that would
  otherwise be ambiguous; the compiler may permit them when the
  dependency graph is well-defined.
- At type-declaration time, attr defaults and recurrent initial
  values may reference same-instance cells (via `self.X`),
  same-type consts, module-level cells (signals, deriveds,
  recurrents, consts), and compile-time-evaluable expressions.
  Cross-instance references are resolved only at placement time,
  not at type declaration.

Traps during initial evaluation (signal initializers, attr defaults,
recurrent initial values, or initial derived evaluation) follow
§13.13.1 — the process aborts. There is no recovery path for traps
encountered during startup.

**Cross-instance init cycles.** When a cell's initial value references
a cell on another instance (e.g., a sibling part's attr), the init
dependency graph includes cross-instance edges. Cycles across
instances are compile errors at the same severity as within-instance
init cycles, identifying the participating instances and cells.

#### 13.2.7 No mutation of cells from Ductus source

Ductus source has no syntactic form for assigning to a signal,
attr, recurrent, derived, or const after declaration. Source-level
expressions read reactive cells and consts; they do not write to
them.

Writes occur only through:

- The host API (`kernel.write_signal`, `kernel.write_attr`,
  `kernel.transaction`) per §13.14. The host cannot directly write
  to recurrents, deriveds, or consts at runtime; influence is
  indirect via signals and attrs.
- Placement-time initial values for attrs and recurrents
  (per §13.8.2). Consts are *not* settable at placement.
- The kernel's own evaluation of `derived` expressions, which
  writes the derived's output cell with the newly computed value.
- The kernel's own evaluation of arm expressions on `recurrent`
  cells, which commits the computed value at the end of the
  publish cycle (per §13.2.4.1 and §13.10.2).

Consts are immutable for the kernel's lifetime: their values are
fixed at compile time and never change. The "no source-level
write" rule applies to all five declaration kinds uniformly.
Ductus programs describe the reactive graph; they do not
imperatively modify it from within.

#### 13.2.8 The `Signal[T]` type

`Signal[T]` is the umbrella type for any reactive cell whose
value type is `T`. It is a first-class type usable in parameter
positions, return types, and generic arguments.

**Subkinds.** Three reactive declaration kinds produce values of
`Signal[T]`:

- `signal X = init` — host-writable `Signal[T]`. Host pushes
  values via `kernel.write_signal` (§13.14.2).
- `derived X = expr` — projected `Signal[T]`. Kernel maintains
  the value consistent with its inputs.
- `recurrent X: T = init | on triggers: expr` — memoryful
  `Signal[T]`. Kernel advances per the arm's expression when an
  arm fires.

The keyword `signal` is overloaded with the type `Signal[T]`:
the keyword declares one specific subkind (the writable cell);
the type covers all three subkinds. This overload is documented
here and elsewhere referenced as "the `Signal[T]` type" vs "a
`signal` declaration" to disambiguate.

**Where `Signal[T]` is used:**

- **Operator parameters** (§13.17) — operators take `Signal[T]`
  to bind to a reactive cell at instantiation, allocating
  internal state tied to that cell.
- **Operator return types** — operators return `Signal[T]`
  representing their output cell.
- **Function parameters** — `fn` may accept `Signal[T]` as a
  parameter type. The compiler distinguishes call-site semantics
  by the function's declared signature: a `fn(x: T)` parameter
  receives the cell's current value (with reactive transparency
  per §13.12.2); a `fn(s: Signal[T])` parameter receives the
  cell reference. No call-site syntactic difference; resolution
  is by type.

`Signal[T]` is read-only when received as a parameter. There is
no source-level form for writing to a `Signal[T]` value (the
no-mutation rule of §13.2.7 applies). The cell may still be
written by the host (for `signal` subkind) or by the kernel (for
`derived` and `recurrent` subkinds), but not through the
`Signal[T]` reference itself.

**Generics.**

`Signal[T]` is parametric. Generic functions and operators may
abstract over the value type:

```
operator passthrough[T](source: Signal[T]) -> Signal[T]:
  source

fn observe[T](signal: Signal[T]) -> string:
  // some debugging utility, etc.
  ...
```

Standard trait bounds apply (§3.1, §5.1). The constraint
`Signal[T: Numeric]` requires T to satisfy `Numeric`.

**Reading a `Signal[T]` field on records.**

A reactive cell may have a record value: `Signal[Record]`.
Field access on the cell's value is reactive — `cell.field`
inside a derived expression projects the field, and the derived
re-evaluates whenever the cell's value changes (any field). This
is coarse-grained: changes to one field invalidate consumers of
all other fields. For finer granularity, project early into
stable derived cells, expose distinct cells from the source, or
use a **reactive composite** (§13.2.9) to give each field its own
cell within a record/tuple/array shape.

#### 13.2.9 Reactive composites

A **reactive composite** is a record, tuple, or fixed-array binding
whose fields or elements are independently reactive. Reactive
composites address the coarse-grained limitation noted in §13.2.8
(where a `Signal[Record]` re-evaluates all consumers on any field
change) by giving each reactive field or element its own cell while
preserving the composite's record/tuple/array type at the type-system
level.

##### 13.2.9.1 Form

Reactive composites are constructed in any reactive declaration
context by binding individual fields or elements to reactive sources,
static values, or reactive expressions. The composite's type is the
underlying record, tuple, or fixed-array type — no new type qualifier
is introduced (per §13.2.9.10).

**Records:**

```
type PeakResult:
  some_property: f32
  some_other_property: f32
  some_regular_property: i32

derived A = PeakResult(
  some_property: signal_a,
  some_other_property: signal_b,
  some_regular_property: 15,
)
```

**Tuples:**

```
derived t = (signal_a, 15, signal_b)
// t: (f32, i32, f32)
```

**Fixed arrays:**

```
derived arr: f32[4] = [signal_a, signal_b, 0.0, signal_c]
```

The same forms apply in `attr` declarations on node and connection
instances. Use in `signal` and `recurrent` declarations is
constrained by their host-write and arm-update semantics
respectively; see §13.2.1, §13.2.4 for the underlying constraints.
The most natural fit is `derived`.

##### 13.2.9.2 Per-field reactivity model

The composite is a structural grouping; it does not have its own
outer cell. Each field or element is independently reactive based on
its binding form (§13.2.9.3). When a constituent reactive cell
updates, only that field is dirty — consumers reading other fields
through the same composite are not invalidated.

This distinguishes reactive composites from the §13.2.8
`Signal[Record]` case, where the entire record value is one cell and
any field change invalidates all consumers of any field. Reactive
composites are the recommended construct when fine-grained per-field
update propagation matters.

##### 13.2.9.3 Field binding form

The form of each field's right-hand side determines that field's
reactive status:

| RHS form                                  | Field becomes                       |
|-------------------------------------------|-------------------------------------|
| Bare reactive name (signal/attr/derived/recurrent) | Alias to that cell — no new cell |
| Reactive expression                       | Implicit derived cell (§13.2.9.4)   |
| Literal or compile-time constant          | Static field — no cell, embedded constant |
| Non-reactive value expression             | Static field — evaluated once at startup |

**Bare-name aliasing.** `some_property: signal_a` does not allocate a
new cell. `A.some_property` *is* `signal_a` for all purposes —
including cell identity (§15.4.1.1), hot reload (§13.15.2), and any
type change to the underlying signal on reload.

**Implicit derived cells.** `some_property: signal_a * 2 + signal_b`
allocates a fresh derived cell with ID `A.some_property` (§13.2.9.4).
Dependency edges to `signal_a` and `signal_b` are added to the graph
specification (§15.4). The expression's evaluation rules are
identical to those of an explicit `derived A.some_property = ...`
declaration.

**Static fields.** `some_regular_property: 15` is a compile-time
constant. No reactive cell is allocated; the value is embedded in
the composite's lowered representation per §15.5. Static fields
participate in the composite's value but do not contribute cell
entries to the graph specification.

##### 13.2.9.4 Cell identity and the graph specification

Reactive-expression fields and aliased fields contribute or
reference cell entries in the graph specification (§15.4.1) using
the path syntax of §15.4.1.1:

- Records: `<binding>.<field_name>` (e.g., `A.some_property`).
- Tuples: `<binding>.<index>` (e.g., `t.0`, `t.1`, `t.2`).
- Fixed arrays: `<binding>.<index>` (e.g., `arr.0`, `arr.3`).

Implicit derived cells (§13.2.9.3) contribute a new cell entry at
their composite-field path with the appropriate dependency edges.

Aliased fields do not contribute new cell entries — the alias
target's existing entry is referenced. Hot-reload identity matches
follow the alias target (§13.15.2).

Static fields contribute no cell entries; they appear only in the
composite's lowered value representation.

The composite binding itself (`A`, `t`, `arr`) is a naming prefix,
not a cell. It does not appear as a standalone cell in the graph
specification.

##### 13.2.9.5 Reading a reactive composite

**Field access** reads the corresponding cell (for aliased fields
and implicit derived cells) or returns the embedded constant (for
static fields):

```
derived peak_x: f32 = A.some_property         // reads A.some_property cell
let r: i32 = A.some_regular_property          // returns embedded 15
```

**Whole-composite reads** — passing the composite to a function
parameter typed as the composite's type, returning it from a
function, or binding it to a `let` of the composite's type — do not
allocate a snapshot. Per §13.12.2, function bodies are
reactive-transparent templates; expressions in the body that read
fields of the parameter resolve through to the underlying cells of
the caller's composite:

```
fn report(p: PeakResult) -> string:
  // p.some_property here resolves to A.some_property's cell
  // when this function is reached from a context where p was A
  ...

derived msg: string = report(A)
```

Materialization to a concrete value happens only at the boundaries
of §13.2.9.7.

##### 13.2.9.6 `let` bindings

A `let` binding whose declared (or inferred) type is the
composite's type may name a reactive composite. The binding is an
alias to the same underlying cells; reading through the let-bound
name resolves to the kernel's current cell values, not to a
snapshot taken at let-binding time:

```
fn process(p: PeakResult) -> f32:
  let q = p                         // q aliases p; same cells
  q.some_property * 2.0             // reads p.some_property's cell live
```

This is the composite-typed analogue of the §13.2.8 `Signal[T]`
binding form: when the binding's type is the composite's type, the
binding is structural — it preserves the live cell references of
its RHS. The standard scalar auto-deref rules of §13.2.8 still
apply to single-cell reads (`let v: f32 = A.some_property`
auto-derefs per the existing rules).

**Ownership.** A reactive composite binding names cells held by
the kernel, not stack-owned data; multiple live aliases to the
same composite may coexist without violating §11's single-
ownership rule, just as multiple `Signal[T]` parameters may name
the same cell. Materialization to a concrete value (§13.2.9.7)
produces a `PeakResult`/tuple/array instance subject to the
standard §11 ownership rules from that point on.

##### 13.2.9.7 Materialization boundaries

Reactive transparency through functions and `let` bindings means
reactive composites stay live across most of the language. Three
boundaries force materialization to a concrete value:

- **Storage in non-reactive collections.** Pushing a reactive
  composite into a `Vec`, `Map`, or analogous container materializes
  the current per-field values:

  ```
  // vec: Vec[PeakResult]
  let vec2 = vec.push(A)             // A materialized at push time;
                                      // vec2 holds a concrete snapshot.
  ```

  The pushed element is a concrete snapshot; subsequent changes to
  the underlying cells do not propagate to `vec2`'s contents.

- **FFI handoff to host code.** Any value crossing into the host
  via the Host API (§13.14) is materialized; host code does not see
  reactive cells.

- **Serialization and persistence.** Hot reload state save, debug
  dumps, and any explicit serialization path materializes
  composites to concrete values.

Within Ductus source code outside these boundaries, reactive
composites remain live.

##### 13.2.9.8 The `with` expression in reactive contexts

The `with` expression (§6.1.5) extends to reactive composites
without syntactic change. Field overrides in a `with` applied
within a reactive declaration context follow the per-field binding
rules of §13.2.9.3:

```
derived A2 = A with some_regular_property: signal_c
// A2.some_regular_property is now aliased to signal_c;
// A.some_regular_property remains the static value 15.

derived A3 = A with some_property: 0.0
// A3.some_property is now a static 0.0;
// A.some_property remains aliased to signal_a.

derived A4 = A with some_property: signal_a * 0.5
// A4.some_property is an implicit derived cell;
// A.some_property remains aliased to signal_a.
```

The result of `with` is a new reactive composite binding with its
own per-field reactive shape. The base composite is unchanged. Each
`with`-produced binding has its own cell IDs (§15.4.1.1) for any
fields that contribute cells.

The interpretation of a `with` RHS — alias, implicit derived,
static — depends on the **binding form**, not the RHS syntax alone:

- A reactive declaration (`derived A2 = base with field: signal_c`,
  `attr a = base with ...`) produces a reactive composite per the
  rules of §13.2.9.3; `signal_c` aliases as a reactive field.
- A plain `let` binding to the `with` expression's result, not
  itself flowing into a reactive declaration, produces a concrete
  value per the standard §6.1.5 semantics — `signal_c` is read for
  its current value at the let-binding's evaluation and the result
  is a concrete `PeakResult`.

##### 13.2.9.9 Distinction from nodes

Reactive composites are data-only. They have no placement, no
participation in node/connection topology, no lifecycle beyond the
declaration that introduces them, and no `recurrent` or behavioral
content. Nodes (§13.3) provide the full instance machinery —
hierarchical placement, connections, hot-reload identity at the
instance level — and remain the appropriate construct when behavior
or topology is needed.

The two have overlapping flavor (both expose per-field reactive
cells) but distinct purposes: nodes are *runtime entities* in the
reactive graph; reactive composites are *value forms* that group
reactive cells under a record, tuple, or fixed-array shape.

##### 13.2.9.10 Type system

A reactive composite's type is the underlying record, tuple, or
fixed-array type — `PeakResult`, `(f32, i32, f32)`, `f32[4]` in the
examples above. There is no `reactive PeakResult` qualifier; the
type system does not distinguish reactive composites from
non-reactive values of the same type. Reactivity is a property of
the *binding* (recorded in the graph specification per §15.4) and of
the per-field cells, not of the type.

This means a function `fn report(p: PeakResult) -> string` works
identically whether called with a reactive composite binding or a
concrete `PeakResult` value. The reactivity is invisible at the
type signature; transparency flows through (§13.12.2). The
distinction between live and snapshotted access is determined by
the caller's context, not by the function's signature.

#### 13.2.10 The `Node[T]` type

`Node[T]` is the type of a node **specification** — a placement
expression captured as a value, whose later invocation by a
receiving node instantiates a node whose `default attr`
(§13.2.2.1) accepts a `T`. `Node[T]` values are first-class:
they may appear as attr types, function parameters, return
types, and generic arguments.

A `Node[T]` value is constructed by writing a placement
expression in a position expecting `Node[T]`. The placement
syntax is identical to inline placement (§13.8.5.2); the only
difference is the *context* — when used as an attr value, the
placement is captured as a specification rather than performed
in-line.

```
node ItemHost:
  attr item: Node[Post]            // accepts a Node[Post] specification

ItemHost host | item=PostItem/some_post   // PostItem/some_post is the Node[Post] value
```

Here `PostItem/some_post` is a `Node[Post]` value — a placement
specification of a `PostItem` whose `default attr` is bound to
`some_post`. The specification is *deferred*: it is not evaluated
when the attr is set, but invoked by the receiving node (`ItemHost`)
when it chooses to instantiate.

##### 13.2.10.1 What can be placed as a `Node[T]`

Any node type `N` declaring `default attr d: T` (per §13.2.2.1) is
a valid `Node[T]` value via the placement form `N/expr`. Nodes
without a `default attr` cannot be used as `Node[T]` values in v1;
future spec revisions may add a form for binding non-default attrs
via the same mechanism.

`T` is the type accepted at the `default attr` position. A node
`PostItem` with `default attr post: Post` produces a `Node[Post]`
value when written as `PostItem/<expr>`.

##### 13.2.10.2 Lifetime and identity

A `Node[T]` value held in an attr is a specification, not an
allocated instance. Cell allocation happens only when the receiving
node instantiates the specification — typically once at startup,
producing one set of template cells with paths under the receiving
node's path (e.g., `host.item.<template_field>`). Drop of the
receiving node drops the template cells per §14.8.

For *child-placement-style* external supply with cardinality, list
semantics, and possible per-instance scoping (the pattern used by
`Repeat`, §13.5.4), the `parts:` clause and §13.5 keyed-scope
primitive are the appropriate mechanism — not `Node[T]` attrs.
`Node[T]` is for attr-shaped *singular* template slots; `parts:`
is for child-placement slots with cardinality.

##### 13.2.10.3 Restrictions

- `Node[T]` values cannot be read in user expressions or evaluated
  for their structure; they are consumed only by receiving nodes
  that know how to instantiate them.
- A `Node[T]` attr cannot be `mut` and cannot be written to from
  Ductus source after the attr is set (per §13.2.7).
- A `Node[T]` value's captured references (e.g., to exposed attrs
  of the receiving node via its placement name) are bound by
  reference; per-invocation, the receiving node updates those
  references and re-invokes the specification.
- Generic constraints on `T` behave as standard generic bounds
  (§3.1, §5.1).

### 13.3 Nodes

#### 13.3.0 Concept

A node is a reactive entity — a composable unit that holds values
(attrs, recurrents), computes values (deriveds), and communicates
with other nodes through typed connections. Each node type is a
nominal type with a body declaring its members. Each placement of
a node type creates an instance with its own cells.

Composition takes two forms:

- *Containment* — sub-nodes are placed inside a parent node as
  *parts* (§13.4). The parent owns the parts; their lifetimes are
  bound to the parent's.
- *Communication* — nodes communicate with each other through
  *connections* (§13.6). Connections are typed directed links
  carrying their own reactive content; they are not passive
  pointers.

The reactive graph is the running structure of all node instances
and the connections between them. Once constructed, the graph's
shape is fixed (§13.1, "Static graph").

Nodes are distinct from records (§6): records are pure data values
that exist anywhere in a program; nodes are reactive entities that
exist only as placed instances in the graph, with per-instance
reactive cells managed by the kernel.

#### 13.3.1 Declaration

```
node TypeName[GenericParams]?:
  satisfies Trait1, Trait2                            // optional trait conformance
  parts: Type1, Type2                                 // optional permitted part types
  in: Conn1, Conn2                                    // optional incoming connection types
  out: Conn3, Conn4                                   // optional outgoing connection types
  when: predicate                                     // optional activation predicate (§13.9)
  const name: Type = value                            // per-type compile-time constants
  signal name: Type = initial                         // per-instance runtime-fed entry points
  attr name: Type = default                           // per-instance user-configured cells
  default attr name: Type = default                   // positional default attr (at most one; §13.2.2.1)
  recurrent name: Type = init | on t1: expr           // per-instance memory cells
  derived name: Type = expr                           // per-instance reactive values
  stream policy[N] name: Type = source                // per-instance event sequences (§13.18)
```

All body items are optional. A node with no attrs, no deriveds, no
parts, and no connections is legal but typically unused.

```
node Driver:
  satisfies Drivable
  out: Drives
  attr expertise_level: i32 = 5
  attr risk_tolerance: f32 = 0.5
  derived is_aggressive: bool = self.risk_tolerance > 0.7
```

#### 13.3.2 `satisfies` clause

A node may declare trait conformance via `satisfies` (§3.2). Trait
methods are implemented via `fulfill` blocks (§3.3); node bodies
themselves do not contain `fn` declarations. Functions on node
instances are free functions taking the node type as a parameter,
callable via uniform call syntax (§3.4).

```
trait Displayable:
  fn display(value: Self) -> string

node Driver:
  satisfies Displayable
  attr expertise_level: i32
  attr risk_tolerance: f32

fulfill Displayable for Driver:
  fn display(d: Driver) -> string:
    "Driver(exp: {d.expertise_level}, risk: {d.risk_tolerance})"
```

#### 13.3.3 `parts` clause

```
parts: Type1 [cardinality]?, Type2 [cardinality]?, ...
```

The `parts:` clause is **optional**. Its presence determines what
kinds of child node instances may be placed inside instances of
this node at placement time:

- **No `parts:` clause** — the node accepts child instances of *any
  node type*. Inside the node body, only the heterogeneous
  `self.parts` form is available, and it requires an explicit trait
  bound on the iteration variable (`for p: SomeTrait in self.parts`)
  per §13.4.1. Type-bulk (`self.parts.<NodeType>[i]`) and
  cardinality-bounded forms are not available.
- **With a `parts:` clause** — the node accepts only children whose
  types appear in the listed set, with the declared cardinality
  constraints. Both heterogeneous (`self.parts`) and type-bulk
  (`self.parts.<NodeType>[i]`) access are available; cardinality
  is enforced at placement.

The clause does not place specific instances — it only constrains
what types and how many of each are permitted. The actual children
appear at placement (§13.8.3).

```
-- Restricted parts with cardinality:
node Synthesizer:
  parts: Oscillator+, Filter [=1], Amplifier?
  attr master_volume: f32 = 1.0
```

In this example: at least one Oscillator (`+`), exactly one Filter
(`[=1]`), at most one Amplifier (`?`).

```
-- Open parts (any node type accepted):
node Processor:
  -- no `parts:` clause; accepts any node as a child
  out: WiresTo
```

`Processor` accepts any node type as a part. Inside its body, only
`self.parts` (heterogeneous iteration) is available; the host walks
the parts externally based on its own conventions (e.g., per-type
dispatch via const discriminators — §13.2.5).

A node may have parts of its own type (self-recursion) when `parts:`
is omitted or when the node's own type appears in the `parts:`
clause. Self-recursive placements terminate because each placement
is an explicit user act — the compiler walks finite placement trees,
not infinite type recursions.

##### 13.3.3.1 Cardinality forms

Cardinality may be written as a sigil or a bracketed range. Sigils
cover common cases:

- (no sigil, no bracket) — `0..` (zero or more, unlimited)
- `?` — `0..=1` (optional)
- `+` — `1..` (at least one)
- `!` — exactly one (shorthand for `[=1]`)

Bracketed range forms support arbitrary bounds:

- `[=N]` — exactly N
- `[N..=M]` — between N and M (inclusive on both ends)
- `[N..]` — at least N (no upper bound)
- `[..=M]` — up to M (lower bound 0)

A part type may carry exactly one cardinality specifier (sigil OR
bracket, not both); duplicate specifiers are a compile error.

Sigils attach directly to the type name with no intervening
whitespace: `Type?`, `Type+`, `Type!`. Bracket forms may optionally
have a space before the bracket: `Type[=1]` and `Type [=1]` are both
valid.

##### 13.3.3.2 Access from inside the node body

Parts of a given type are accessible as `self.parts.<NodeType>`,
which is a structural iterable of compile-time-known length range:

- Indexed access: `self.parts.<NodeType>[i]` — legal at type-level
  expressions iff `i < min_cardinality` of that part type.
  Example: under `parts: Oscillator+`, `self.parts.Oscillator[0]`
  is legal (at least one is guaranteed) but `[1]` is not.
- Type-bulk iteration: `for o in self.parts.<NodeType>: ...`
  always works.
- Heterogeneous iteration: `for p in self.parts: ...` iterates
  all parts of all declared types (§13.4.2).

A node without a `parts` clause may still contain children of any
node type (per §13.3.3); inside its body, only the heterogeneous
`self.parts` form is available, and it requires an explicit trait
bound on the iteration variable (`for p: SomeTrait in self.parts`)
per §13.4.1 — type-bulk and cardinality-bounded forms are not
available. A node with a `parts` clause may contain children at
runtime according to the declared cardinality.

#### 13.3.4 `in` and `out` clauses

```
in: ConnType1 [cardinality]?, ConnType2 [cardinality]?, ...
out: ConnType3 [cardinality]?, ConnType4 [cardinality]?, ...
```

The `in` and `out` clauses list the *types* of connections in which
instances of this node may participate as endpoints, with optional
cardinality constraints. `in` connections target this node (the
node is the `to` endpoint); `out` connections originate from this
node (the node is the `from` endpoint). See §13.6 for connection
declarations and §13.8.4 for connection placement.

Cardinality syntax is identical to that of `parts:` (§13.3.3.1):
sigils (`?`, `+`, `!`) or bracketed ranges (`[=N]`, `[N..=M]`,
`[N..]`, `[..=M]`). Default (bare) is unlimited (`0..`).

```
node Driver:
  out: Drives [=1], MaintainedBy?
  in: SponsoredBy [..=3]
```

##### 13.3.4.1 Access from inside the node body

Connections of a given type are accessible as `self.in.<ConnType>`
and `self.out.<ConnType>`, both structural iterables of compile-
time-known length range:

- Indexed: `self.in.<ConnType>[i]` and `self.out.<ConnType>[i]` are
  legal iff `i < min_cardinality` of that connection type.
  Example: under `out: Drives [=1]`, `self.out.Drives[0]` is legal.
- Type-bulk iteration: `for c in self.out.<ConnType>: ...` always
  works.

The access syntax is symmetric with parts (§13.3.3.2): three
namespaces (`parts`, `in`, `out`), each grouping cells by declared
type.

#### 13.3.5 Generic parameters

A node may declare generic parameters in the standard `[T, U, ...]`
form. Generic parameters are in scope within the body's attr,
recurrent, derived, const, parts, and connection declarations:

```
node Buffer[T: Numeric]:
  attr capacity: usize = 16
  attr fill_level: usize = 0
  derived utilization: f32 =
    self.fill_level as f32 / self.capacity as f32

  parts: BufferSlot[T]
```

Each instantiation of `Buffer` with a different concrete `T`
produces a distinct node type with its own cells. Monomorphization
follows §2.3.

#### 13.3.6 No methods in node body

A node body does not contain `fn` declarations. Behavior associated
with a node type lives as free functions whose first parameter is
the node type, or as `fulfill` blocks implementing trait methods.
Calls are made via uniform call syntax per §3.4.

This separation enforces the "node bodies are declarative" rule:
nodes describe structure and reactive content; functions and
methods are imperative computation, distinct in kind.

#### 13.3.7 Exposition (the `expose:` clause)

The `expose:` clause declares the node type's **structural output**
— the list of `Node[T]` placements the kernel traverses when it
encounters an instance of this type. The clause is the node's
"return value" in the structural sense: it determines what an
external reader (and the kernel) sees as the node's content.

```
node TypeName:
  satisfies SomeTrait
  parts: SomeA, SomeB
  in: ConnIn1
  out: ConnOut1
  expose:
    SomeA
    SomeB
  attr foo: i32
  signal user_name: string = "world"
  derived greeting: string = "hello " ++ self.user_name
```

The canonical clause order is: `satisfies` → `parts:` → `in:` →
`out:` → `expose:` → cell declarations.

##### 13.3.7.1 Content

The body of `expose:` is a list of placements — each entry is a
`Node[T]` value, with the same syntax as inline child placements
elsewhere (§13.8). Entries reference:

- A part of self by type-bulk access (`self.parts.SomeA` — the full
  list of supplied parts of that type, in placement order).
- A named part instance (`self.osc1` — see §13.4.1) — when the
  exposition needs a specific named child rather than all parts of
  a type.
- A wrapper placement that contains parts as its own children. The
  wrapper is a node-internal type the exposition uses for structural
  composition:

  ```
  node MyContainer:
    parts: Item
    expose:
      SomeInternalWrapper:
        self.parts.Item
  ```

  Here `SomeInternalWrapper` is a wrapper node whose body contains
  the supplied `Item` children. Internal nodes used this way are
  declared (in stdlib or user code) and accept children via their
  own `parts:` clause.

Each entry in `expose:` may carry a per-placement `when` gate
(§13.9) for conditional activation. Conditionality inside
exposition uses the same `when` clause mechanism that applies
elsewhere — no new control-flow syntax is introduced.

##### 13.3.7.2 Default

When `expose:` is omitted, the node's exposition defaults to
`expose: self.parts` — the kernel traverses all supplied parts in
declaration order. When the node has no `parts:` clause and no
`expose:` clause, the exposition is empty (the node has no
structural output and exists only for its state and connections).

##### 13.3.7.3 External access via `.exposition`

The exposed list is readable from outside the node via the reserved
`.exposition` field: `instance.exposition` returns the list of
`Node[T]` values the instance currently exposes. This is the same
content the kernel traverses; external readers and the kernel see
identical output.

Inside the node body, `self.exposition` is the same list. The
field is read-only; the exposition is fixed by the type's `expose:`
clause (and the placer's supplied parts), not mutable at runtime.

##### 13.3.7.4 Kernel traversal

The kernel traverses what `expose:` produces, not the `parts:`
clause directly. This is the load-bearing distinction:

- **`parts:`** is the constraint and supply mechanism — declares
  what child types are accepted, with cardinality; placement-time
  child placements fill the parts (§13.4, §13.8.3).
- **`expose:`** is the structural-output mechanism — declares which
  parts (and/or wrapping internal nodes containing them) participate
  in the kernel's traversal of this instance.

A node may receive parts that its exposition does not include — for
example, a node may accept administrative or diagnostic parts that
are queried only via the host API, not traversed by the kernel. In
practice the default `expose: self.parts` covers the common case
where every supplied part is exposed.

##### 13.3.7.5 Connections and exposition

Connections (§13.6) are **not** part of exposition. Connections are
instance-to-instance edges, placed at the instantiation site of the
nodes they connect. They are not declared in any node's `expose:`
clause and do not appear in `instance.exposition`.

The motherboard analogy: the parts a motherboard exposes (RAM
slots, CPU socket, expansion slots) are the structural surface of
the board. The wires connecting those parts to each other and to
external components are connections — held by the parts, owned by
no single one, traversed by signals rather than by structural
descent.

### 13.4 Parts

#### 13.4.0 Concept

"Part" is a *role*, not a separate type. A part is a child node
instance placed inside a parent node at construction time. Parts
exist for *containment* (§13.3.0 framing): a parent node may own
child nodes whose lifetimes and addressing are bound to the parent.

Parts vs. top-level placements: a node placed at the module top
level is an independent instance addressable by its name. A node
placed as a part is contained within a parent instance and
addressable only through that parent (e.g., `parent.osc1` or
`parent.parts.Oscillator[0]`). The structural distinction matters
for ownership, hot-reload diffing, and addressing — both kinds of
instances have reactive cells that participate in dependency
graphs, but a part's cells are reachable through the parent's
`self.parts.<Type>` mechanism, whereas a top-level instance is
reachable only by its module-scope name or through connections.

Use parts when:

- The contained instance is conceptually "owned" by the parent
  (a Synthesizer owns its Oscillators; a Form owns its Fields).
- The parent's reactive expressions need to aggregate over the
  child instances (a Synthesizer summing oscillator outputs).
- The compositional structure is part of the parent's identity
  (the Form's fields define the Form).

Use top-level placements when the instance stands on its own and
participates in the graph through connections rather than
containment.

The parent declares the types of children it accepts via its
`parts:` clause (§13.3.3) with optional cardinality; the specific
instances appear via placement (§13.8.3).

**Kernel traversal goes through `expose:`, not through `parts:`
directly.** The `parts:` clause is the constraint and supply
mechanism — declared types, cardinality, and placement-time
filling. The `expose:` clause (§13.3.7) is the structural output
the kernel walks; it references parts (via `self.parts.<Type>` or
by named instance), possibly wrapping them in internal nodes.
Parts that the exposition does not include are not traversed by
the kernel — they remain queryable via the host API and addressable
within the parent's own reactive expressions, but they do not
contribute to the structural descent.

#### 13.4.1 Access forms

Parts of a parent instance are accessible in three ways. The
available access forms depend on whether the parent's `parts:`
clause is declared:

- **Heterogeneous:** `self.parts` — a structural iterable over all
  parts of the parent, regardless of their types.
    - When `parts:` is declared, the iteration variable is typed as
      the sum of the listed types. The body must compile for every
      listed type (per the heterogeneous iteration rules of §13.4.2).
    - When `parts:` is omitted, the iteration variable's static type
      cannot be inferred from the declaration alone (any node type
      may have been placed). The body must declare an explicit trait
      bound on the iteration variable (`for p: SomeTrait in
    self.parts: ...`); the compiler verifies at each placement
      that every placed part type satisfies the bound.
- **Type-bulk (`parts:` declared only):** `self.parts.<NodeType>` —
  a structural iterable over all parts of the given type. Length
  range is determined by the declared cardinality. Available only
  when `<NodeType>` appears in the `parts:` clause.
- **Named individual:** `self.<name>` (or `paramName.<name>` from
  outside the node body) — accesses a specific part by its
  placement-time name. Names are assigned in the placement body
  (§13.8.3) and visible wherever the placement scope is known.
  Available in both forms (with or without `parts:`).

Summary table:

| Form                         | `parts:` declared | `parts:` omitted                 |
|------------------------------|-------------------|----------------------------------|
| `self.parts.<Type>`          | available         | not available                    |
| `self.parts` (unbounded)     | available         | not available (need bound)       |
| `self.parts` (trait-bounded) | available         | available (trait bound required) |
| named (`self.<name>`)        | available         | available                        |

Inside the parent's own type body (its `derived` and `recurrent`
expressions), only type-bulk and heterogeneous forms are available;
placement names aren't visible at the type-declaration level.
Named individual access becomes available in:

- Function bodies receiving a specific instance, where the
  instance's placement names are visible (e.g., `c.osc1.output`
  where `c` is a Composite parameter).
- Other instances' placement bodies that reference the named
  instance.
- The same placement body where the part is declared (subsequent
  lines may reference the just-named part by name).

All three access forms are compile-time resolved; the graph is
static (§13.1), so the compiler knows every part's identity, type,
and placement-name.

#### 13.4.2 Iteration over parts

A function body that receives the parent node as a parameter may
iterate its parts using a `for` loop, accessed via the parameter
name (developer-chosen, not `self`).

**Type-bulk iteration:**

```
fn total_output(s: Synthesizer) -> f32:
  mut sum: f32 = 0.0
  for o in s.parts.Oscillator:
    sum = sum + o.output
  sum

node Synthesizer:
  parts: Oscillator+
  derived total: f32 = total_output(self)
```

`o` has the concrete type `Oscillator` in each iteration. The
compiler unrolls the loop to one reference per declared Oscillator
part.

**Heterogeneous iteration:**

```
fn render_all(c: Composite):
  for p in c.parts:
    p.render()

node Composite:
  parts: Oscillator+, Filter [=1], Amplifier [=1]
```

Inside the body, `p` is typed as the sum `Oscillator | Filter |
Amplifier` (the union of all declared part types). The compiler
unrolls the loop to one body copy per part instance, dispatching
the `p.render()` call statically based on the concrete type. The
body must compile for every part type that appears; if `render`
is unavailable on any part type, the unroll-copy fails at the
for-loop site.

**Heterogeneous iteration with an explicit trait bound:**

```
for p: Renderable in c.parts:
  p.render()
```

The explicit form enforces that all part types implement
`Renderable` at the iteration site (clearer error messages). The
unbounded form (`for p in c.parts`) checks the same constraint
implicitly through body operations.

**Heterogeneous iteration with `match`:**

```
fn process(c: Composite):
  for p in c.parts:
    match p:
      Oscillator(o): o.synthesize()
      Filter(f): f.process()
      Amplifier(a): a.amplify()
```

`p`'s sum type permits regular pattern matching. The compiler
unrolls per part instance and simplifies the match at compile time
so only the matching branch survives in each copy.

Match exhaustiveness rules apply: if the match omits a declared
part type and has no wildcard arm, it is a compile error.

#### 13.4.3 Reactive dependency tracking through parts

When a function called from a reactive expression iterates parts,
each part's reactive cells contribute to the calling expression's
dependency set. In the example above:

- `total_output(self)` reads `p.output` for each part.
- Each `p.output` is a derived on the part.
- The `Synthesizer.total` derived's dependency set includes every
  part's `output` derived.
- When any one part's `output` changes, `total` is dirty.

This works because dependency tracking is provenance-based (§13.12.1):
the compiler tracks reactive cells read by an expression,
transitively through function calls.

#### 13.4.4 Restrictions

- Parts are bound to placement-time names. A node may contain at
  most one part of each name; multiple parts of the same type with
  different names are permitted (subject to the cardinality
  declared in the `parts:` clause).
- Parts are not added or removed at runtime (except via hot reload).
- For heterogeneous iteration (`for p in self.parts`), the body
  must compile for every declared part type (§13.4.2). The optional
  explicit trait bound form (`for p: Trait in self.parts`) gives
  clearer error messages and enforces the constraint at the
  iteration site.

#### 13.4.5 Heterogeneous parts — example

Putting type-bulk, heterogeneous, and named individual access
together:

```
node Composite:
  parts: Oscillator+, Filter [=1], Amplifier [=1]
  derived total_oscillation: f32 = sum_oscillators(self)
  derived processed: f32 = process(self)

fn sum_oscillators(c: Composite) -> f32:
  mut sum: f32 = 0.0
  for o in c.parts.Oscillator:        -- type-specific iteration
    sum = sum + o.output
  sum

fn process(c: Composite) -> f32:
  let raw = c.parts.Oscillator[0].output      -- indexed, legal under `+`
  let filtered = c.parts.Filter[0].apply(raw) -- indexed, legal under `[=1]`
  c.parts.Amplifier[0].amplify(filtered)

-- Placement with optional names:
Composite c1:
  Oscillator osc_a
  Oscillator osc_b
  Filter flt1
  Amplifier amp1

-- Named individual access from outside the type body:
fn debug(c: Composite) -> string:
  "first oscillator: {c.osc_a.output}, filter: {c.flt1.kind}"
```

Three access patterns coexist: `c.parts.Oscillator[i]` (type-bulk
indexed, bounded by cardinality), `for p in c.parts: ...`
(heterogeneous), and `c.osc_a` (named individual, requires the
caller to know placement names).

### 13.5 Template Scopes and Keyed Instantiation

Some stdlib host nodes (like `Repeat`, §13.5.4) instantiate a child
*template* — a part placed in their body — zero, one, or many
times, each instantiation backed by its own state cells. This
section specifies the **keyed-scope primitive** that standardizes
how the kernel manages per-instantiation state and how such host
nodes drive instantiation.

The primitive is **runtime-implementable**: any conformant kernel
exposes the three operations of §13.5.1 to its stdlib host nodes;
the stdlib's documented hosts (the canonical user `Repeat` in
§13.5.4, and future siblings such as `Conditional` and `Switch`)
are layered on top of these operations.

#### 13.5.1 The primitive

For each template-typed parts entry on a host instance — a part
declared in the host's `parts:` clause that the host's exposition
(§13.3.7) treats as a per-instantiation template — the kernel
exposes three operations. The bound template (the part supplied at
the host's placement site) is fixed for the host's lifetime; the
operations therefore parameterize only the **key**.

- **`scope_obtain(key)`** — return the scope for `key`, allocating
  from the host's per-template pool if absent. Newly-allocated
  scopes initialize the template's state cells to their declared
  initial values (per §13.2.6).
- **`scope_drop(key)`** — drop scope `key`: invoke `Drop` (§14.8)
  on its state cells in reverse declaration order; return the pool
  slot.
- **`scope_evaluate(key)`** — evaluate the template's deriveds and
  any recurrent arm bodies eligible to fire within scope `key`'s
  state context. References to `self` inside the template body
  resolve to scope `key`'s cells; references to the host's own
  attrs (e.g., via the host's placement name per §13.4.1) resolve
  to the host instance's cells.

The host is responsible for sequencing these operations correctly:
typically `scope_obtain` for new keys, `scope_evaluate` for active
keys, and `scope_drop` for keys no longer active.

##### 13.5.1.1 Per-template pool

Each template-typed parts entry on a host instance has its own
keyed pool. The pool's element shape is the template type's
**state-shape** (§13.5.2). The pool's index space is the host's
key domain. Scopes are independent — no cell sharing across keys.

Pool sizing follows the §14.3.5 extensible-pool model: pools grow
as keys are added and shrink as keys are dropped, subject to the
kernel's pool-management policy.

#### 13.5.2 State-shape and the no-pool optimization

A template's **state-shape** is the set of stateful cells it
declares:

- `signal` declarations inside the template's body (§13.2.1).
- `attr` declarations inside the template's body (§13.2.2).
- `recurrent` declarations inside the template's body (§13.2.4).

`derived` declarations are *not* part of the state-shape; they are
pure functions of state cells and the host's exposed attrs.
`const` declarations are static and not state.

**When the state-shape is empty** (the template declares only
deriveds, or no body cells at all), the kernel allocates **no pool**
for the host's template entry. `scope_obtain(key)` becomes a no-op,
`scope_drop(key)` is a no-op, and `scope_evaluate(key)` evaluates
the template's deriveds against the host's exposed attrs without
any per-key state context.

This is the **stateless-template fast path**: data-driven
multiplicity with O(1) cost per data change beyond per-element
derived evaluation. Programs that use only stateless templates
incur no per-key allocation overhead.

The compiler determines a template's state-shape at compile time
and statically selects between the pool and no-pool case per
host-template instantiation.

#### 13.5.3 Hot reload and cell identity

When state-shape is non-empty, per-key state cells participate in
hot reload per §13.15.2's cell-identity rules. The cell path
follows §15.4.1.1:

```
<host_placement_path>.<key>.<template_field>
```

Keys are required to be stringifiable primitives (the exact bound
is specified per host node; §13.5.4 specifies it for `Repeat`).
The key value serves as the path component.

Hot-reload changes to the template's body follow the standard
reload-safe / reload-unsafe rules of §13.15.4, applied uniformly
across all live keys.

#### 13.5.4 Repeat: data-driven multiplicity

The stdlib provides `Repeat`, the canonical template-hosting node
for iterating over a `Signal[T[]]` source. `Repeat` declares a
single template-typed part (`Item`); the placer supplies the
template by placing it in `Repeat`'s body. `Repeat` uses the
§13.5.1 primitive directly: it sequences `scope_obtain`,
`scope_drop`, and `scope_evaluate` per its iteration semantics.

##### 13.5.4.1 Signature

```
node Repeat[T]:
  default attr source: Signal[T[]]
  parts: Item!                           // exactly one template part
  attr key: fn(T, usize) -> K            // K: StringifiableKey, inferred

  attr current: T                        // kernel-updated per iteration
  attr index: usize                      // kernel-updated per iteration
  attr first: bool                       // kernel-updated per iteration
  attr last: bool                        // kernel-updated per iteration
  attr count: usize                      // kernel-updated per iteration

  expose: self.parts.Item
```

`K` is the key function's return type, inferred at placement. `K`
must satisfy the `StringifiableKey` trait — the stdlib trait
admitting `i8`–`i64`, `u8`–`u64`, `bool`, `char`, `string`. When
`key` is omitted, the stdlib default returns `index` unchanged and
`K` is `usize`.

- `source` is the iterated signal (default attr; set via `/expr`
  per §13.8.5.2).
- `parts: Item!` declares that placements may supply exactly one
  part of type `Item`. The Item type is what the placer provides
  at the `Repeat` body — the template that `Repeat` invokes per
  source element. The Item's underlying node type must declare a
  `default attr` whose type is `T` (so `/ref.current` can bind it
  at placement).
- `key` is a function from `(element, index)` to a stringifiable
  primitive.
- `current`, `index`, `first`, `last`, `count` are attrs the
  template references via Repeat's placement name (§13.4.1). The
  kernel updates these as part of Repeat's iteration semantics
  (§13.5.4.2); they are not host-writable.
- `expose: self.parts.Item` declares the exposition (§13.3.7):
  the kernel traverses the supplied Item template. Repeat's
  kernel-aware iteration semantics drive that traversal per
  source element with §13.5.1's scope operations.

##### 13.5.4.2 Iteration

Whenever `source` is dirty, the kernel:

1. Computes the key for each element via the `key` function.
2. Keys in `old ∩ new` carry over: their scopes are preserved per
   §13.5.1; exposed attrs (`current`, `index`, etc.) are updated
   for re-invocation.
3. Keys in `old − new` are dropped: `scope_drop(key)` is invoked.
4. Keys in `new − old` are added: `scope_obtain(key)` is invoked.
5. For each key in `new` in element order, the kernel updates
   exposed attrs and calls `scope_evaluate(key)`.

Reordering elements in `source` without changing the key set
performs no scope allocations or drops — only the exposed attrs
(`index`, `first`, `last`) update.

##### 13.5.4.3 Use

```
node PostItem:
  default attr post: Post
  attr expanded: bool = false
  derived title: string = self.post.title

UI app:
  signal posts_data: Post[] = []
  Repeat ref/posts_data:
    PostItem/ref.current
```

`Repeat` iterates `posts_data`; for each post, the kernel
`scope_obtain`s a scope keyed by index (since `key` is omitted),
binds `ref.current` to that post, and `scope_evaluate`s the
template. `PostItem`'s `expanded` cell is allocated per-key.

For reordering-stable state, supply `key` on Repeat's placement:

```
Repeat ref/posts_data | key=post_id_key:
  PostItem/ref.current
```

where `post_id_key` is `fn(p: Post, _: usize) -> i64` returning
`p.id` (defined as a free function or stdlib utility).

##### 13.5.4.4 Cell identity

Per the §13.5.3 path rule, the per-key cell for `PostItem`'s
`expanded` attr in the example above (with `posts_data` containing
a post whose key evaluates to `42`) is at path
`app.ref.42.expanded`.

When the template is **stateless** (its state-shape is empty per
§13.5.2), no per-key cells are allocated.

##### 13.5.4.5 Hot reload

`Repeat`'s per-key cells follow §13.5.3 / §13.15.2. A change to
the `key` function is reload-unsafe at the per-instance level
(§13.15.4): old keys may not match new ones, so per-instance
restart of the `Repeat` is required. The kernel diagnoses this
and performs the restart cleanly.

##### 13.5.4.6 Performance

- Stateless template (state-shape empty per §13.5.2): O(1) per
  data change beyond per-element derived evaluation. No
  allocation.
- Stateful template, element add/remove: O(K) per data change
  where K is the number of added/removed keys (key-set diff plus
  per-key cell init/drop).
- Stateful template, reorder: O(1) per moved element. State
  follows key; no allocation or drop.

Programs that do not use `Repeat` incur no runtime cost from its
machinery; the cost model is "pay for what you iterate."

##### 13.5.4.7 Restrictions

- The template's underlying node type must declare `default attr
  d: T` so that `/expr` at the template's placement (e.g.,
  `PostItem/ref.current`) binds correctly.
- The template references Repeat's exposed attrs via the
  placement name; closure-over-outer-state beyond §13.12's
  reactive transparency is not supported.
- Nested `Repeat` instances are permitted but each requires a
  distinct placement name to avoid path collisions (§13.5.4.4).
- The `key` function must be reactive-pure: no dependencies
  beyond `(item, index)` per §13.12. This guarantees key
  stability across evaluations.

### 13.6 Connections

#### 13.6.0 Concept

A connection is a directional, typed link between two node
instances. Connections are first-class entities — they have identity,
reactive content (attrs, recurrents, deriveds), and may implement
traits. They are not passive references; they are active channels
through which nodes communicate.

Why first-class types: connections carry reactive state *about the
relationship* rather than about either endpoint. A `Drives`
connection between a Driver and a Vehicle holds attrs like
`aggressiveness` that belong to neither node individually but to
the act of driving. Connections also satisfy traits (like
`Circularity`, §13.6.5) that govern static graph properties.

Communication direction: every connection has a *source* (the
`from` endpoint) and a *destination* (the `to` endpoint). A
connection participates in the source node's outgoing surface
(declared via `out:`) and the destination node's incoming surface
(declared via `in:`).

A node declares which connection types it can participate in via
its `in:` and `out:` clauses (§13.3.4), with optional cardinality
constraints. The actual connection instances appear at placement
(§13.8.4).

**Connections and exposition.** Connections are not part of any
node's `expose:` clause (§13.3.7); they are not structural output.
A connection is held by its endpoint nodes but owned by no single
one — it lives at the instance graph level, traversed by signals
rather than by the kernel's structural descent. The motherboard
analogy: parts compose into the board (`expose:`); wires between
parts are connections (instance-to-instance edges held by, but
not contained within, the parts they connect).

Connection vs. node-typed attr: a node could in principle hold a
direct reference to another node (e.g., `attr target: SomeNode`),
but this offers no place to carry per-relationship state, no static
guarantees about graph topology, and no trait conformance for cycle
handling. Connections solve all three: they carry state about the
relationship, give the type system structural information for
compile-time graph analysis, and integrate with traits.

#### 13.6.1 Declaration

A connection declares its endpoint types in one of three forms:
single, cartesian, or pairs. A connection uses exactly one form;
mixing forms (e.g., `pairs:` alongside `from:`+`to:`) is a compile
error.

##### 13.6.1.1 Single form (one from-type, one to-type)

```
connection TypeName[GenericParams]?:
  satisfies Trait1, Trait2                            // optional trait conformance
  from: SourceType                                    // required, exactly once
  to: DestType                                        // required, exactly once
  when: predicate                                     // optional activation predicate (§13.9)
  const name: Type = value                            // per-type compile-time constants
  signal name: Type = initial                         // per-instance runtime-fed entry points
  attr name: Type = default                           // per-instance writable cells
  default attr name: Type = default                   // positional default attr (at most one; §13.2.2.1)
  recurrent name: Type = init | on t1: expr           // per-instance memory cells
  derived name: Type = expr                           // per-instance reactive values
  stream policy[N] name: Type = source                // per-instance event sequences (§13.18)
```

A connection type may declare a `default attr` per §13.2.2.1. At
placement, `/expr` targets the connection's default attr (§13.8.5.1);
the destination endpoint is supplied separately in the placement's
body, not via `/expr`.

Example:

```
connection Drives:
  from: Driver
  to: Drivable
  attr enhanced_handling: bool = false
  attr aggressiveness: f32 = 0.5
  derived effective_speed: f32 =
    self.to.top_speed * (self.from.expertise_level as f32 / 10.0)
```

`from` and `to` are not attributes — they are endpoint slots,
first-class structural elements of every connection. Attribute
syntax (placement-time `name=value` settings via the attribute
clause, flags) does not target them.

Inside the body, `self.from` and `self.to` resolve to the endpoint
instances directly (their concrete types).

##### 13.6.1.2 Cartesian form (multiple from-types and/or to-types)

```
connection TypeName:
  from: TypeA, TypeB, ...
  to: TypeX, TypeY, ...
  // body declarations (when, const, signal, attr, recurrent, derived, stream) per §13.6.1.1
```

All cartesian combinations of from-types × to-types are valid
placements. Inside the body, `self.from` is the sum type of all
listed from-types, and `self.to` is the sum type of all listed
to-types. Pattern matching is required to extract the concrete
endpoint types.

Example:

```
connection Owns:
  from: Person, Company
  to: Vehicle, Property
  attr acquired_at: i64
  derived display: string = match (self.from, self.to):
    (Person(p), Vehicle(v)): "{p.name} owns car {v.id}"
    (Person(p), Property(pr)): "{p.name} owns property {pr.id}"
    (Company(c), Vehicle(v)): "company {c.name} owns car {v.id}"
    (Company(c), Property(pr)): "company {c.name} owns property {pr.id}"
```

The compiler requires the match to be exhaustive over the cartesian
product (4 combinations in this example).

##### 13.6.1.3 Pairs form (constrained from-to combinations)

```
connection TypeName:
  pairs:
    FromType1 -> ToType1
    FromType2 -> ToType2
    ...
  // body declarations (when, const, signal, attr, recurrent, derived, stream) per §13.6.1.1
```

Only the listed pair combinations are valid placements. Inside the
body, the endpoints are accessed via `self.pair`, a sum type whose
variants correspond to the declared pairs.

Example:

```
connection Drives:
  pairs:
    Driver -> Vehicle
    Racer -> Boat
  attr aggressiveness: f32 = 0.5
  derived speed: f32 = match self.pair:
    (Driver(d), Vehicle(v)): v.top_speed * (d.expertise as f32 / 10.0)
    (Racer(r), Boat(b)): b.knots * r.aggression
```

In pairs form, `self.from` and `self.to` are not independently
accessible — endpoints must be extracted via `self.pair` and
pattern matching. This reflects the semantic coupling: pair-form
connections enforce that specific from-types pair with specific
to-types.

Rules for pairs form:

- Duplicate pairs (same `From -> To` listed twice) are a compile
  error.
- Asymmetric pair counts are allowed; pair uniqueness, not type
  count, is what matters. `pairs: A -> X; A -> Y; B -> Y` is legal
  (A can go to X or Y; B only to Y).
- All attrs/deriveds in the body are uniform across pairs. If
  pair-conditional content is needed, declare a separate connection
  type. (Pair-conditional content would require trait-like
  machinery; deferred to potential v2+.)

A connection body does not contain `fn` declarations, paralleling
node bodies (§13.3.6).

#### 13.6.2 `from`, `to`, and `pair` references in expressions

The endpoint access inside a connection body depends on the form
of its declaration (§13.6.1):

- **Single form** (`from: X / to: Y`): `self.from` is typed as `X`
  directly; `self.to` is typed as `Y` directly. Attrs and deriveds
  of the endpoints are accessible via `self.from.attr_name`,
  `self.to.attr_name`, etc.
- **Cartesian form** (`from: X, Y / to: A, B`): `self.from` is the
  sum `X | Y`; `self.to` is the sum `A | B`. Pattern matching
  against the sums (typically as a tuple `(self.from, self.to)`)
  is required to extract concrete endpoint types.
- **Pairs form** (`pairs:`): `self.pair` is the sum of declared
  (FromType, ToType) tuples. Pattern matching against `self.pair`
  extracts the concrete pair. `self.from` and `self.to` are not
  independently available in pairs form.

`self.from`, `self.to`, and `self.pair` are bound at the
connection's *placement* time. Each placement specifies its source
(the enclosing instance) and destination (a bare-identifier reference
in the placement's body, §13.8.5.1). Inside the connection type's
body, these identifiers resolve to those specific instances.

#### 13.6.3 Generic connections

Connections may declare generic parameters:

```
connection Contains[T]:
  from: Container[T]
  to: T
  attr index: usize = 0
```

Generic parameters scope over the connection's `from`, `to`, attrs,
recurrents, and deriveds. Each unique instantiation produces a
distinct connection type per §2.3.

#### 13.6.4 No methods in connection body

A connection body does not contain `fn` declarations. Functions on
connections are free functions taking the connection type, dispatched
via uniform call syntax. Trait methods are implemented in `fulfill`
blocks. Same rule as nodes (§13.3.6).

#### 13.6.5 The `Circularity` trait

A connection type may declare conformance to the `Circularity` trait
— a language-defined marker trait (§3.7.4) — to indicate that
placements of this connection type may participate in topology cycles
in the node graph (§13.11.2).

```
trait Circularity                          -- marker trait, no methods

connection MyDelayed:
  satisfies Circularity
  from: Clip
  to: Clip
```

The compiler enforces a static rule: every topology cycle in the
construction-time node graph must traverse at least one connection
whose type satisfies `Circularity`. Cycles consisting only of
non-`Circularity` connections are compile errors.

Its sole purpose is to opt a connection type into participation in
cycles.

The decision of which connection types satisfy `Circularity` is
domain-defined. A connection type whose runtime semantics introduce
a temporal break between source and destination (e.g., a connection
that says "destination plays after source finishes") may safely
satisfy `Circularity`, since cycles through such connections cannot
loop instantaneously. A connection type whose semantics imply
simultaneity (e.g., "destination plays alongside source") should
*not* satisfy `Circularity`, since cycles through such connections
would imply infinite simultaneous activation.

### 13.7 The `self` Keyword

`self` is a context-restricted keyword that resolves to the instance
currently being declared or constructed.

#### 13.7.1 Scope

`self` is available only inside the body of a node or connection
declaration. Specifically, in:

- Attr default expressions: `attr x: i32 = self.other_attr + 1`.
- Recurrent initial-value expressions: `recurrent x: i32 = self.other_attr | ...`.
- Recurrent arm expressions: `... | on tick: self.x + 1`.
- Derived expressions: `derived y: bool = self.x > 0`.
- Iteration over parts in reactive expressions inside a node body:
  `for p in self.parts: ...`. Inside free functions that receive
  the node as a parameter, the parameter name (developer-chosen)
  is used to refer to the instance, not `self`.

`self` is *not* available in:

- Record or enum body declarations.
- Trait declarations (use the capitalized `Self` for the type-level
  identifier per §3.1.1).
- Free function bodies, including functions whose first parameter
  is a node or connection type. Such functions use the parameter's
  name to refer to the instance.
- Module top-level scope.

```
node Driver:
  attr expertise_level: i32 = 5
  attr risk_tolerance: f32 = 0.5
  derived skill_factor: f32 = self.expertise_level as f32 / 10.0
                                   //  ^^^^ self inside node body — valid

fn aggressive(d: Driver) -> bool:
  d.risk_tolerance > 0.7        // function uses parameter name, not self
```

#### 13.7.2 Resolution and reactive dependencies

A reference through `self` to an attr, recurrent, or derived
participates in the reactive dependency graph in the usual way.
`derived x: f32 = self.y + 1` depends on `self.y`; when `self.y`
changes, `x` becomes dirty.

For each *instance* of the type, `self` resolves to that specific
instance. The compiler emits dependency edges per-instance: instance
`A` of `Driver` has a `skill_factor` cell whose dependency set
includes instance `A`'s `expertise_level` cell, not the cell of
some other Driver instance.

#### 13.7.3 Self vs Self (lowercase vs capitalized)

The capitalized `Self` is the type-level identifier used in trait
declarations and `fulfill` blocks (§3.1.1). It refers to the
implementing type, not an instance.

The lowercase `self` is the instance-level identifier used in node
and connection bodies. It refers to a specific instance at
runtime.

The two are distinct: `Self` is a type-system concept usable only
in type positions; `self` is a value usable only in expression
positions inside node/connection bodies. They never overlap.

### 13.8 Placement

*Placement* is the syntax for instantiating nodes, parts, and
connections into a concrete reactive graph. It is distinct from
value construction of records (which uses constructor syntax per
§6.1.3).

#### 13.8.1 Top-level instances

A top-level placement creates a named instance of a node type at
module scope:

```
Driver john_doe | expertise_level=10 risk_tolerance=0.8:
  Drives | enhanced_handling=true aggressiveness=0.8: some_car
```

The first line is `TypeName instance_name` followed (optionally) by
attribute settings (§13.8.7) and (optionally) by `:` introducing a
body of child placements (§13.8.3, §13.8.4). The syntax is identical
to internal part placements (§13.8.3); the only distinction is that
top-level placements *must* declare a name (internal parts may
omit the name when not referenced from outside the placement).

Instance names are unique within their declaring scope. Two
top-level placements with the same name in the same module is a
compile error.

#### 13.8.2 Setting attrs and recurrent initial values

Attrs and recurrent initial values are set via inline attribute
syntax on the placement line. The body of a placement is reserved
exclusively for child placements (§13.8.3, §13.8.6); attribute
settings do not appear in the body.

A single-line placement with attrs uses one leading `|` followed by
one or more `name=value` settings separated by whitespace:

```
Driver john_doe | expertise_level=10 risk_tolerance=0.8

Counter c1 | count=100
```

A multi-line placement keeps the first attribute on the placement's
main line; subsequent attributes continue on lines indented exactly
to the column of the first attribute (no further `|` characters):

```
Driver john_doe | expertise_level=10
                  risk_tolerance=0.8
                  license_class="full"
```

A placement with attrs *and* children combines the forms: attributes
inline (or via aligned continuation), then `:` introducing the
body:

```
Driver john_doe | expertise_level=10 risk_tolerance=0.8:
  Drives | enhanced_handling=true: some_car
```

The named cell must be declared on the placed type as either an
`attr` or a `recurrent`. Setting any other identifier — including
`signal`, `derived`, or `const` declarations — is a compile error.
The value's type must match the cell's declared type (subject to
the standard widening rules).

##### 13.8.2.1 Reactive vs. compile-time placement values

The right-hand side of an attribute setting at placement may be:

- A **compile-time expression** — a literal, a `const` reference, a
  compile-time-evaluable computation. The value is fixed at
  placement and stored directly in the attr's cell.
- A **reactive expression** — references reactive cells (signals,
  attrs, recurrents, deriveds) visible at the placement scope:
  sibling part instances by name, top-level signals or consts, or
  any cell reachable through visible names. The placement creates
  an implicit `derived` bridging the source cells to the target
  attr, so the attr reactively tracks changes to the source.

```
App my_app:
  Fetch fetcher / "url"
  Log / fetcher.response                  // reactive binding: Log's default attr
                                           // tracks fetcher.response

App other_app:
  Counter c1
  Display d1 | label=format(c1.count)     // reactive: d1.label tracks c1.count,
                                           // formatted as a string
```

Mechanically, a reactive placement value introduces a synthesized
derived in the parent's scope; the target attr is bound to that
derived. When any cell in the expression's provenance changes
(§13.12.1), the synthesized derived re-evaluates and the target
attr updates.

The compiler determines reactivity from the expression's provenance
set: any reference to a reactive cell makes the expression
reactive; otherwise the expression is compile-time and the value is
fixed at placement.

##### 13.8.2.2 Restrictions

- **`const` declarations are not settable at placement.** A const's
  value is fixed by its declaration on the type; placement bodies
  cannot override it. Attempting to set a const at placement is a
  compile error.
- **`signal` declarations are not settable at placement.** Signals
  receive their values from the host/runtime, not from placement
  syntax. Their declared initial value applies at construction;
  subsequent values come through the host API (§13.14.2).
- **Recurrent initial-value overrides accept only compile-time
  values.** Unlike attrs, the placement form for recurrents
  (`count=100`) does *not* accept reactive expressions. A
  recurrent's initial value is a fixed compile-time constant at
  construction; runtime advancement happens via the recurrent's
  arms (§13.2.4).

For boolean attrs, the same value may also be set via flags
(§13.8.8). The two mechanisms (`name=value` / `name` / `!name`
inline form, and flag form) target the same underlying attr cells;
setting the same attr via two mechanisms is a compile error
(duplicate-set).

Reactive bindings apply to the **`name=value` inline form** for
attrs. Flag form has no expression slot — a flag always sets a
literal boolean (true for `'name`, false for `!name`) — so reactive
bindings do not apply to flags.

A type's `default attr` (§13.2.2.1) — when declared — is
additionally settable via the positional `/expr` form (§13.8.5). The
rule is uniform across nodes and connections: `/expr` targets the
`default attr`. Connection destinations are supplied in the
placement's body, not via `/expr` (§13.8.5.1).

The attribute clause and flags do *not* target consts. Consts
cannot be overridden at placement (§13.8.2.2). Recurrent initial
values are overridable via the same `name=value` form in the
attribute clause, but only with compile-time-evaluable expressions
(no reactive bindings).

For recurrent cells, only the initial value is overridable at
placement. The arm structure (triggers, guards, and `next_expr`
expressions) is a structural type property and cannot be overridden
per-instance (§13.2.4.3).

If a cell is not set at placement, its declared default (for attrs)
or declared initial value (for recurrents) applies. Consts always
have their type-declared value.

#### 13.8.3 Child parts

The body of a placement (the indented block introduced by `:`) is
reserved exclusively for child placements — parts and connections.
Attribute settings on the enclosing instance do not appear in the
body; they live on the placement's main line via inline attribute
syntax (§13.8.7) or aligned multi-line continuation (§13.8.2).

```
Component chip_b | label="B":
  Pin out1                                // child part (Pin instance named out1)
  Pin in1                                 // another child part
```

A child placement that names a node type listed in the parent's
`parts:` clause is a part. The placement creates an instance of
that node type as a child of the parent.

The optional instance name (`out1`, `in1` in the example) is the
*placement-time name* of the part. Once named, the part is
accessible by that name from contexts where the placement scope is
visible:

- Inside the same placement body: `out1` refers to the just-placed
  Pin (useful when subsequent connection placements need to
  reference it).
- Outside the parent type, in function bodies receiving the parent
  instance: `chip_b.out1` (where `chip_b` is the parameter name)
  accesses the named part directly, in parallel with the type-bulk
  form `chip_b.parts.Pin[i]`.
- In other instances' placement bodies that reference this
  instance, by qualified path: `chip_b.out1`.

Named individual access is the placement-time companion to the
type-bulk and heterogeneous access forms described in §13.4.1.
Names are not available inside the parent's own type body (the
type declaration doesn't know what placements will exist) — within
the parent type, use `self.parts.<NodeType>[i]` or `self.parts`
instead.

Cardinality declared in the parent's `parts:` clause (§13.3.3.1) is
enforced at placement: the number of placed parts of each type
must satisfy the declared cardinality. Violations are compile
errors at the placement site.

#### 13.8.4 Connections

A connection placement creates a directional edge from a source
instance to a destination instance. The placement is written inside
the source instance's body. **The source is always the immediately
enclosing instance** — the instance whose body directly contains the
placement line. There is no special prefix or sigil: the source is
determined positionally.

```
App my_app:
  Fetcher fetcher / "url"                       // part placement
  WiresToExternal: external_target              // source = my_app

  Filter filter / "low-pass":
    Cascade: next_filter                        // source = filter
    WiresTo | gain=0.5: monitor                 // source = filter
  Filter next_filter / "high-pass"
  Monitor monitor
```

`WiresToExternal: external_target` is placed in `my_app`'s body, so
its source is `my_app`. `Cascade: next_filter` and
`WiresTo | gain=0.5: monitor` are placed in `filter`'s body, so
their source is `filter`. The rule is uniform across nesting depth;
the depth at which the connection appears does not change how the
source is determined.

The connection type must match a type listed in the source instance's
`out:` clause (or in the type's traits' contributions).

The destination is supplied in the connection placement's body as a
single bare-identifier reference (§13.8.5.1). The `/expr` slot, when
present, sets the connection's `default attr` (§13.2.2.1); the
attribute clause (`| name=value …`) sets named attrs. None of these
target the destination.

A placement-level `when` modifier may be attached to gate the
connection instance (§13.9). The modifier appears in the inline-parts
ordering between `/Expr` and the attribute clause (§13.8.9), before
the body's `:`:

```
// (presumes Filter declares signal_active and App declares debug_enabled)
App my_app:
  Filter filter / "low-pass":
    Cascade when self.signal_active: next_filter      // gated on filter's own attr
  Filter next_filter / "high-pass"
  Monitor monitor
  WiresTo when self.debug_enabled: monitor            // gated on my_app's attr
```

**Scope of placement-level `when`.** The `when` predicate evaluates
in the scope of the enclosing source instance, not the
connection-being-placed. The connection has not yet been constructed;
its `self` is unavailable. To reference the connection's own attrs in
a gate, use a type-level `when:` clause inside the connection type's
body (§13.6.1.1) instead.

`self` in the predicate resolves to the enclosing source instance.
In `Cascade when self.signal_active: next_filter` (inside `filter`'s
body), `self.signal_active` is `filter.signal_active`. In
`WiresTo when self.debug_enabled: monitor` (inside `my_app`'s body),
`self.debug_enabled` is `my_app.debug_enabled`.

##### 13.8.4.1 Terminology

Connections are not "owned" by either endpoint. A connection is an
*edge* between two instances: it is *initiated from* its source (the
enclosing instance whose body contains the placement) and
*terminated at* its destination (the bare-identifier reference in
the placement's body). "Source" and "destination" are the canonical
terms; "owner" is not.

#### 13.8.5 The `/expr` form

The `/expr` form appears immediately after the placed type name
(and any flags), before any optional instance name and before the
attribute clause (§13.8.7). The expression after `/` is the
*positional argument* of the placement: it targets the placed type's
`default attr` (§13.2.2.1), whether the placed type is a node or a
connection. Using `/expr` on a type without a declared `default attr`
is a compile error.

##### 13.8.5.1 For connection placements

For a connection placement, `/expr` sets the connection type's
`default attr` (when declared); the destination endpoint is supplied
in the placement's body as a single bare-identifier reference to an
existing instance whose type satisfies the connection's `to:` clause
(§13.6.1.1).

```
// connection Drives: from: Driver; to: Drivable; default attr aggressiveness: f32 = 0.5

Drives: some_car                          // no /expr; destination only
Drives/0.8: some_car                      // /expr sets default attr; destination in body
Drives | enhanced_handling=true: some_car // attr clause + destination
Drives/0.8 | enhanced_handling=true:      // /expr + attr clause + multi-line body
  some_car
```

The destination is a bare identifier resolving to a named instance in
scope. Inline placement specs as destinations are not supported in v1;
a future revision may admit them in the same body slot. The
syntactic shape (`:` followed by a single placement) leaves room for
that extension without further syntax changes.

##### 13.8.5.2 For node (part) placements

For a node placement (typically a part placed inside a parent),
`/expr` sets the type's `default attr` (§13.2.2.1). The expression
must match the default attr's declared type.

```
node Log:
  default attr message: string

Program p1:
  Log /"Hello World"
  Log /"System ready"
```

Each `Log /"..."` placement creates a Log part with its `message`
attr set to the string. Equivalent inline form:

```
Program p1:
  Log | message="Hello World"
  Log | message="System ready"
```

A node-placement `/expr` form requires the type to have a declared
`default attr`. Using `/expr` on a node type without a `default
attr` is a compile error.

##### 13.8.5.3 Summary

The `/expr` form is positional shorthand for the type's `default
attr` (§13.2.2.1):

- On both nodes and connections, `/expr` targets the type's
  `default attr` (when declared).
- Connection placements additionally supply the destination as a
  single bare-identifier reference in the placement body, introduced
  by `:` (§13.8.5.1).

#### 13.8.6 Disambiguation summary

Both node and connection placements may have a body, but the body's
content differs by placement kind:

- **Node placement body** — the indented block after `:` on a node
  placement line — contains child placements: parts and connections
  (§13.8.3, §13.8.4). Multiple children allowed; same-line
  multi-placement uses commas per §13.8.10.
- **Connection placement body** — the indented block (or inline
  single-line form) after `:` on a connection placement line —
  contains exactly *one* bare-identifier reference: the destination
  instance (§13.8.5.1). No child placements, no inline placement
  specs, no attr settings, no multiple values.

**Attribute settings do not appear in the body** of either
placement kind; they live on the placement's main line via the
attribute clause (`| name=value …`, §13.8.7) or aligned multi-line
continuation per §13.8.2.

The identifiers `to` and `from` are reserved as endpoint slots inside
connection *type* bodies (§13.6.1.1); they cannot be used as attr
names on connections, and they do not appear in connection
*placement* bodies.

A single line of a node placement body may contain multiple child
placements separated by commas (§13.8.10). A placement that
introduces its own children body via `:` cannot share its line with
sibling placements; multi-line layout is required when both same-line
siblings and `:`-introduced children are needed.

The parser distinguishes attribute settings from placements
lexically: attribute settings appear after a single leading `|` on
the placement's main line (or on aligned continuation lines) and use
`name=expr` form; placements use the placement form per §13.8.9.

#### 13.8.7 Attribute clause

After the `TypeRef` (and optional flags, instance name, and `/expr`
slot) of any placement, an attribute clause may follow on the same
line, introduced by exactly **one leading `|`**. After the leading
`|`, attributes are written one after another separated by
whitespace; intermediate `|` characters between attributes are not
permitted.

Three syntactic forms within the attribute clause:

```
name=value         -- set attribute `name` to expression `value`
name               -- set boolean attribute `name` to true (bare form)
!name              -- set boolean attribute `name` to false
```

```
Sensor s1 | gain=0.5 active !calibrated
```

Parentheses may be used freely around values for grouping or
disambiguation:

```
Sensor s1 | gain=(base + offset) active
```

When the attribute clause extends across multiple lines, the
continuation lines have no `|` and are aligned exactly to the column
of the first attribute on the placement's main line:

```
Sensor s1 | gain=0.5
            active
            !calibrated
```

Multi-line continuation does not change semantics — it is purely a
formatting variant.

Setting the same attribute twice on one placement is a compile error
(duplicate-set, parallel to the rule for record-field
duplicate-set).

Attribute settings target *attrs* declared on the placed type
(directly or inherited via satisfied traits). They do not target
recurrent, derived, signal, or const declarations — targeting a
non-attr identifier is a compile error. The expression in
`name=value` must match the attr's type subject to standard widening
rules. The boolean-true (`name`) and boolean-false (`!name`) forms
require the attr to be of type `bool`; non-boolean attrs used with
those bare forms are a compile error.

The expression in `name=value` may be a compile-time constant *or*
a reactive expression, per §13.8.2.1. A reactive expression creates
a synthesized derived bridging the source cells to the target attr.
All three forms (value, bare, negated-bare) and the flag form
(§13.8.8) handle attr binding uniformly.

#### 13.8.8 Flags

A *flag* is a single non-letter character appearing adjacent to a
placed type's `TypeRef` (no intervening whitespace), aliasing a
boolean attribute of the type. **Flags apply uniformly to node and
connection placements** — any boolean attr on a node or connection
type may be annotated with `@flag` and set via the flag form at
placement.

```
Pin' p1                                // ' is a flag on Pin (node placement)
Component?* c1                         // two flags: ? and *
WiresTo'! my_wire: chip_b.in1          // two flags on a connection placement
```

Flags are declared on attr declarations via the `@flag('c')`
annotation:

```
node Pin:
  @flag('!')
  attr reverse_polarity: bool = false

  @flag('\'')
  attr is_power: bool = false

connection WiresTo:
  from: Component
  to: Pin
  @flag('\'')
  attr enhanced_signal: bool = false
  @flag('!')
  attr reverse_polarity: bool = false
```

The annotation argument is a `char` literal per §9.1.2. Only boolean
attrs may carry `@flag`; non-boolean attrs with `@flag` are a
compile error.

##### 13.8.8.1 Flag character set

The permitted flag characters are:

```
' ! ? * + ^ ~ @ $
```

Each is a non-letter character not part of identifier syntax. (`#`
is a valid identifier character per §1.4 and is therefore excluded
from the flag set.)

##### 13.8.8.2 Flag-character uniqueness

Within a type's effective attribute surface (its own attrs plus
those inherited via satisfied traits), each flag character must be
unique. Two attrs claiming the same flag character is a compile
error at the type declaration site, identifying both attrs.

##### 13.8.8.3 Flag semantics

At a placement site, each flag character in the run resolves to the
boolean attr it aliases, setting that attr to `true`. There is no
flag form for setting `false`; users who need to override a
default-`true` attr to `false` use the inline `!name` form (§13.8.7).

The asymmetry — flags set true only — is deliberate. Flags are for
the *unusual* case; the default should be chosen so most placements
omit the flag.

##### 13.8.8.4 Flag/operator disambiguation

Several flag characters double as operator tokens elsewhere in the
language:

- `'` is both a flag and the opener of a `char` literal (§9.1.2).
- `?` is both a flag and the postfix Try operator (§8.4).
- `@` is both a flag and the annotation prefix (`@derive`).
- `!` is both a flag and the boolean-NOT operator.

Disambiguation is positional: in placement position, a non-letter
character immediately following the `TypeRef` path (no intervening
whitespace) is a flag-run opener. In any other position
(expression context, annotation context, etc.) it is the operator.

```
Pin' p1                            // flag run after TypeRef (placement context)
let c: char = '\''                 // char literal in expression context
let r = some_fallible()?           // postfix Try in expression context
@derive(Eq) type Point:            // annotation prefix in declaration context
  ...
```

##### 13.8.8.5 No duplicate-set across forms

A boolean attr may be set via at most one mechanism per placement:
the flag form, or the inline `name` / `!name` / `name=value` form
(§13.8.7). Using two mechanisms on the same attr in one placement
is a compile error.

```
Pin' p1 | reverse_polarity=false    // ✗ duplicate: ' flag and inline both target reverse_polarity
```

The diagnostic class is the same as duplicate-set for inline
attributes (§13.8.7).

#### 13.8.9 Ordering of inline parts

A placement's inline parts have a fixed order:

```
TypeRef [FlagsRun]? [InstanceName]? [DefaultArgPart (`/Expr`)]? [WhenClause (`when` Pred)]? [AttrClause]? [BodyIntro (`:` Body)]?
```

- Flags immediately adjacent to TypeRef (no whitespace).
- Optional instance name follows the type/flags.
- The `/Expr` default-arg slot follows the name. For both node and
  connection placements, `/Expr` sets the type's `default attr`
  (§13.2.2.1). Permitted only when the placed type declares a
  `default attr`.
- The optional `when` clause follows next. It gates the placement
  (§13.9). The predicate is a boolean expression in placement scope.
  Use `when` to make the placement conditional. When `/Expr` is
  absent (the type has no default attr, or the default value is not
  being overridden), `when` slots immediately after whichever
  preceding element is present.
- The attribute clause (§13.8.7) — a single leading `|` followed
  by attribute settings — follows next.
- The optional body — introduced by `:` — comes last. For node
  placements, the body holds zero or more child placements (parts
  and connections, §13.8.3, §13.8.4). For connection placements,
  the body holds exactly one bare-identifier reference to the
  destination instance (§13.8.5.1, §13.8.6). A `when` predicate
  containing an unparenthesized `:` must be parenthesized to avoid
  colliding with the body-introducer `:`; common predicates are
  flat boolean expressions and do not require parens.

Example (connection placement, default attr + flags + destination):

```
Drives'! my_drive / 0.8 | enhanced_handling: some_car
^^^^^^^                                                  -- TypeRef + 2 flags
         ^^^^^^^^                                        -- instance name
                   ^^^                                   -- /Expr (sets default attr)
                        ^^^^^^^^^^^^^^^^^^^^^            -- attribute clause
                                              ^^^^^^^^^  -- destination in body
```

Example (node placement with `default attr`):

```
Log / "Hello World" | level="info"
^^^                                       -- TypeRef
      ^^^^^^^^^^^^^^                      -- /Expr (sets default attr `message`)
                      ^^^^^^^^^^^^^^^^    -- attribute clause (sets attr `level`)
```

Example (gated connection placement with `when` + body):

```
Debugger d1 / "trace" when self.verbose | level=2: target
^^^^^^^^                                                     -- TypeRef
         ^^                                                  -- instance name
            ^^^^^^^^^                                        -- /Expr (default attr)
                      ^^^^^^^^^^^^^^^^^                      -- when clause (predicate)
                                          ^^^^^^^^^          -- attribute clause
                                                    ^^^^^^^  -- destination in body
```

Example (gated placement, no `/Expr`):

```
Logger when self.debug_enabled
^^^^^^                              -- TypeRef
       ^^^^^^^^^^^^^^^^^^^^^^^^^    -- when clause (no /Expr present)
```

The `/Expr` form requires the placed type to have a declared
`default attr` (§13.2.2.1) — the same rule for both node and
connection placements. Using `/Expr` on a type without a `default
attr` is a compile error.

#### 13.8.10 Same-line multi-placement

Multiple placements may appear on a single line, separated by
commas. The comma always terminates a placement at the current
scope level — there is no context-sensitive disambiguation:

```
A3, rest, A4                              // three bare placements
G4/4, G5/4                                // two /expr placements
Sensor s1 | gain=0.5, Sensor s2 | gain=0.7  // two attributed placements
```

The comma rule is universal: same-line placements are *always*
comma-separated, regardless of whether the placements have names,
`/expr`, attribute clauses, or any combination. This removes
parser context-sensitivity — the comma is the unambiguous
delimiter.

**A placement that introduces its own children body via `:`
cannot share its line with sibling placements.** Such a placement
owns the rest of its line (and the indented block that follows).
To combine `:`-bearing children with same-line siblings, use
multi-line layout:

```
// ✗ ambiguous and disallowed:
//   SomePart: Child1, Child2, AnotherPart

// ✓ SomePart with three inline children (no siblings on this line):
SomePart: Child1, Child2, Child3

// ✓ Same-line siblings, no `:` children:
SomePartA, SomePartB | attr=1, SomePartC

// ✓ Multi-line — `:`-bearing placement on its own line:
SomePart:
  Child1
  Child2
AnotherPart                                // sibling on next line
```

Same-line multi-placement is opt-in: one placement per line remains
the dominant form. Same-line layout is intended for dense sequences
(e.g., music notation) where vertical compactness aids readability.

### 13.9 Conditional Activation

A *gate* is a boolean predicate that conditions whether a node
instance or a connection instance is *active*. Gates are declared
with the `when` clause. While the predicate is true the instance
participates in propagation; while it is false the instance is
*inactive* and its propagation behavior is constrained per §13.9.7.

Gates are a language feature: the compiler reasons about the graph
under the assumption that gates may open or close at any publish,
and the runtime enforces gate state at edge level. Routing is not a
host concern; it lives in the source.

#### 13.9.1 Concept

A `when` predicate is, for reactive-evaluation purposes, a derived
expression of type `bool`: it follows the same purity rules
(§13.2.3), provenance tracking (§13.12.1), cycle-detection rules
(§13.11.2), and recurrent-read semantics (§13.11.4) as any other
derived. The expression forms accepted are identical. What differs
is the structural role — the predicate's value is consumed by the
kernel to gate propagation through the construct it modifies, not
exposed through a named cell readable by other expressions.

It evaluates in the scope of the construct it modifies: inside a
type body it sees `self.*` and items visible at the type's
declaration scope; inside a placement it sees the full placement
scope.

```
connection Pulse:
  from: Driver
  to: Listener
  when: self.from.is_emitting                 // type-level gate
```

```
App my_app:
  Logger l1 when self.debug_enabled           // placement-level gate
```

Two design moves justify the clause:

- **Host-decided routing is rejected.** If the host chose which
  edges propagate, the compiler could not statically reason about
  reachability, cycles, or the per-publish DAG. The graph would
  become opaque between publishes.
- **A marker trait was rejected.** Using a regular attr name like
  `active` to mean "this is the gate" would reserve a common
  identifier for what is fundamentally a structural concern. The
  `when` keyword takes the role explicitly.

#### 13.9.2 Type-level `when:`

A node or connection body may declare a single `when:` predicate as
a schema member. It uses colon form, consistent with other body
fields (`from:`, `to:`, `attr name:`, `recurrent name:`, etc.):

```
signal trigger: u64 = 0

node OneShot:
  out: Pulse
  recurrent fired: bool = false
    | on trigger: true
  when: not self.fired                        // intrinsic refractory gate

connection ActiveEdge:
  from: Source
  to: Sink
  attr weight: f32 = 1.0
  when: self.weight > 0.0                     // self-conditional gate
```

Type-level gates encode constraints intrinsic to the type — a
refractory period, a debounce, a self-disabling threshold — that
every placement should inherit by default.

A single `when:` clause is permitted per type. Multiple `when:`
declarations in one body are a compile error.

Traits cannot declare or require a `when` predicate. Gates are
per-type structural metadata, not behavior: two types satisfying
the same trait may have different gates (or none at all). A trait
declaration containing a `when:` clause is a compile error.

#### 13.9.3 Placement-level `when`

A placement may attach a `when` modifier to override or introduce a
gate for that specific instance. It uses no colon, consistent with
modifier-style clauses:

```
Logger l1 when self.debug_enabled
Filter f1 / "low-pass" when self.dsp_mode == DspMode::Realtime | gain=0.5
ShowsCount when self.from.count > 0: d1
```

Parts placed inside a parent's body may carry `when` clauses
identically — the same grammar applies to part placements as to
top-level placements:

```
node App:
  parts: Logger [=2], Monitor [=1]
  attr verbose: bool = false
  attr health_checks_enabled: bool = true

App my_app:
  Logger l1                                         // always active
  Logger l2 when self.verbose                       // gated on parent attr
  Monitor m1 when self.health_checks_enabled        // feature flag
```

`l2` and `m1` are constructed unconditionally (the static graph
rule of §13.1 holds — the graph's shape is fixed at compile time).
What `when` controls is propagation: when `self.verbose` is false,
`l2`'s recurrents do not advance, its deriveds do not recompute,
and its outputs do not propagate. Its cells hold their initial
values per Model B (§13.9.7).

Position in the inline-parts ordering is fixed by §13.8.9: after
`/Expr` (if present), before the attribute clause. When `/Expr` is
absent the `when` clause follows whatever element does precede it.

The asymmetric punctuation between type level (`when:`) and
placement level (`when`) reflects the underlying grammatical
distinction. In a declaration body, members are labeled schema
slots; the colon is the labeling marker. At a placement, modifiers
are positional and keyword-introduced; no colon is used.

#### 13.9.4 Predicate type and scope

The predicate must have type `bool`. A non-bool predicate is a
compile error.

Otherwise, a `when` predicate follows normal expression scope
rules — no special restrictions.

- **Type level:** the predicate may reference `self.*` (own cells
  and, for connections, `self.from` / `self.to` / `self.pair`),
  plus anything visible at the type's declaration scope under
  normal visibility rules (module-level signals, consts, imports).
- **Placement level:** the predicate may reference the full
  placement scope — siblings, parent attrs, top-level cells, and
  any other identifier resolvable at that point.

Coupling concerns (a type-level predicate referencing external
state binds the type to that state) are style, not correctness.
The visibility system (`public`, `shared`, `private` — §10.1) is
the mechanism that controls how far that coupling can leak.

#### 13.9.5 Override semantics

A placement-level `when` *replaces* the type-level `when` on that
specific instance. The two predicates are not conjoined and not
stacked — replacement is total:

```
connection Pulse:
  from: Driver
  to: Listener
  when: self.from.is_emitting

App my_app:
  Driver d1
  Listener l1
  Pulse: l1                                           // gate: self.from.is_emitting
  Pulse when self.debug_audio: l1                     // gate: self.debug_audio (overrides type-level)
```

If a placement needs both predicates, the placement-level form must
combine them explicitly:

```
Pulse when self.from.is_emitting and self.debug_audio: l1
```

Override is not implicit conjunction because conjunction would make
type-level gates impossible to relax. Replacement gives the
placement author full control.

A placement with no `when` modifier inherits the type-level `when`,
if any. A placement on a type with no type-level `when` and no
placement-level `when` is unconditional — always active.

#### 13.9.6 Self-conditional gates

A gate predicate may reference cells of the gated instance itself.
The kernel evaluates the predicate against the cells' current
committed values; cyclic self-reference is well-defined: the gate
predicate evaluates against the gated cell's *previously-committed*
values from the prior publish. The gate-open transition is itself a
propagation event scheduled within the publish that flips the
predicate (per §13.9.7's snap-on-gate-open rule).

```
connection WeightedEdge:
  from: Node
  to: Node
  attr weight: f32 = 1.0
  when: self.weight > 0.0                            // self-conditional
```

Type-level self-conditional gates on nodes are likewise allowed
(refractory, threshold, debounce — §13.9.2 example).

#### 13.9.7 Runtime semantics

The runtime model is *Model B — frozen-when-gated, snap on
activation*. The kernel evaluates gate state at edge level on each
propagation cycle. Gated subgraphs do no work; the cost of a
permanently-gated node is the cost of evaluating its `when`
predicate.

**Definitions.**

- A *gate-true* edge propagates normally.
- A *gate-false* edge on a gated *node* does not propagate to the
  destination's output-affecting state, but inbound connections
  still deliver to the gated node's input cells, so the node's own
  `when` predicate can re-evaluate.

**Behavior on a gated node** (its `when` predicate is currently
false):

- **Input cells (`in`):** stay live. Connections delivering into the
  gated node still write their values into the destination's `in`
  cells. This is necessary so a node whose `when` references its
  inputs can wake up.
- **`when` predicate:** re-evaluates whenever any cell in its
  provenance set changes. A flip from false to true is itself a
  propagation event (see below).
- **Recurrents:** do not advance. Their arms do not fire; the
  cells hold their last committed value. Any arm trigger that
  would have fired during a gated period is lost — the kernel
  does not queue triggers, and gate-open does not replay them.
  The recurrent remains at its last committed value until a future
  arm trigger fires during an active period.
- **Deriveds:** do not recompute. They hold their last committed
  value. (An exception: deriveds whose values are read by the
  `when` predicate must remain current; the kernel keeps the
  minimum subgraph needed for predicate evaluation live. This is
  an implementation concern of §14, transparent at the language
  level.)
- **Outputs:** do not propagate. Outbound connections from the
  gated node do not deliver to their destinations.

**Behavior on a gated connection** (the connection's own `when` is
false): a gated connection edge does not propagate at all — its
destination receives nothing through this connection. Note this
differs from a gated *node*, whose inbound connections still deliver
to its `in` cells (the node's own `when` re-evaluates against those
inputs).

**Snap on gate-open.** When a `when` predicate transitions from
false to true between publishes, the kernel treats this as a
propagation event. The frozen cells re-evaluate against current
upstream state in topological order. Any value that would have
propagated during the gated period is re-computed *as of now* (not
replayed); downstream sees the activation as a single jump from
the frozen value to the current value.

This snap may cause discontinuities in domains where smooth value
transitions matter (audio velocity, control voltages). Smoothing
is a separate concern handled by the parameter system, not by the
gate primitive. The gate guarantees correctness, not continuity.

**Cell-value reads on gated subgraphs.** Reads always return a
defined value of type T (no `Option[T]`), because:

- All attrs have values (defaults or required-at-placement —
  §13.2.2).
- All recurrents have initial values (mandatory — §13.2.6).
- All signals have initial values (mandatory — §13.2.6).
- All deriveds compute against always-defined inputs.
- All connection-level deriveds compute against `self.from` which
  always has defined cells.

On a gated node or connection, reads return frozen values: the
last committed value during an active period, or the initial value
if the instance has never been active.

#### 13.9.8 Interaction with the per-publish DAG

The compiler builds the reactive dependency graph (§13.11.1)
independent of gate state — gates do not remove edges from the
static graph, they suspend propagation through edges at runtime.
The per-publish DAG (§13.11.3) is constructed each publish; during
construction, gated edges contribute no dirty propagation to their
destinations' output-affecting cells, but do contribute to input
cells and `when` predicate provenance.

A single delegating note in §13.10.2 records this: edges whose gate
predicate evaluates false do not propagate to destination outputs;
the gate-open transition itself is a propagation event scheduled
within the same publish that flips the predicate.

#### 13.9.9 Interaction with `Circularity`

Gates do not relax the `Circularity` rule (§13.11.5). Every
topology cycle must traverse at least one connection type that
satisfies `Circularity`, regardless of whether any edge in the
cycle is gated. A gated edge is structurally still an edge; gate
state can change at runtime, and the cycle constraint must hold
across all reachable gate configurations.

```
// Forbidden even if Edge has a `when` clause that will be false at runtime
connection Edge:
  from: A
  to: B
  when: false                                    // always closed

// Cycle A → B → A via Edge in both directions is still a topology
// error unless at least one Edge type satisfies Circularity.
```

#### 13.9.10 Hot reload of `when` predicates

Adding, removing, or modifying a `when` predicate is a
reload-safe change (§13.15.3). The predicate is structural
metadata, not cell identity. On reload, the new predicate
participates in the next publish; cells retain their values.
Changes to the predicate that would have caused a state to differ
historically are not retroactive — the new predicate takes effect
prospectively only.

#### 13.9.11 Diagnostics

The compiler emits the following diagnostic classes for `when`
clauses. Concrete wording is implementation-defined; the classes
listed here are normative.

**Non-bool predicate.** The predicate's inferred type is not `bool`.

```
error: `when` predicate must be of type `bool`
  --> connection Foo: when: self.weight
                            ^^^^^^^^^^^ expression has type `f32`
  hint: introduce a comparison (e.g., `self.weight > 0.0`)
```

**Multiple `when:` clauses in a single type body.** Per §13.9.2.

```
error: multiple `when:` clauses in connection body
  --> connection Foo
        first  declared at line 5
        second declared at line 8
  hint: at most one `when:` per type; combine predicates with `and`/`or`
```

**`when:` in a trait declaration.** Per §13.9.2.

```
error: `when:` is not permitted in a trait declaration
  --> trait Drivable: when: ...
  hint: gates are per-type structural metadata, not part of trait contracts
```

**Unresolved reference in predicate.** Standard name-resolution
failure, surfaced in `when`-clause context.

```
error: unknown identifier `self.frobnicate` in `when` predicate
  --> node Foo: when: self.frobnicate
  hint: did you mean `self.activate`?
```

**Cycle through `when` provenance.** Per §13.11.2; gate predicates
participate in cycle detection identically to deriveds.

```
error: instantaneous cycle in reactive expressions
  derived `a.x` depends on `b.gate`
  `when` predicate of `b` depends on `a.x`
  hint: introduce a `recurrent` declaration on the cycle, or
        eliminate the cyclic dependency
```

#### 13.9.12 Stdlib pattern: `When` / `Then` / `Else`

The combination of `parts:` declarations, the `expose:` clause
(§13.3.7), and per-placement `when` gates supports a canonical
stdlib pattern for conditional activation. The stdlib provides
`When`, `Then`, and `Else` as cooperating node types:

```
node When:
  default attr cond: Signal[bool]
  parts: Then!, Else?
  expose:
    self.parts.Then when self.cond
    self.parts.Else when !self.cond
```

`Then` and `Else` are simple stdlib wrapper nodes; each accepts a
single child via its `parts:` slot and re-exposes it (the same
pattern Repeat uses with `parts: Item!` + `expose: self.parts.Item`,
§13.5.4.1).

Placement:

```
When/some_cond:
  Then: SomeDecision
  Else: SomeFallback
```

The placer supplies a `Then` and (optionally) an `Else` as parts
of the `When` instance. The exposition gates each by `cond`: when
`cond` is true the Then's child is active and the Else's child is
inactive; when `cond` is false the roles reverse. The wrapping
types (`Then`, `Else`) carry no other state; they are pure
structural markers that the exposition's `when` gates discriminate.

The same pattern generalizes:

- A two-way `When` with no `Else` is just `When` with `Else?`
  declared and not supplied at placement; the cond-false case has
  no active child.
- `Match` (sum-type-driven, multiple variants) is a future stdlib
  node following the same pattern: `parts:` lists one wrapper per
  variant; `expose:` gates each by the corresponding tag.
- User-defined conditional nodes can follow the same idiom — there
  is no kernel-aware special-casing of `When`/`Then`/`Else`. They
  are documented stdlib types using the general mechanism.

### 13.10 Reactive Evaluation

The kernel processes reactive state via two operations:
**writes** (signal/attr) accumulate dirty bits without evaluation;
**publish** evaluates dirty cells, advances recurrents per their
firing arm expressions, and swaps the back buffer atomically so that
consumers see the new state.

#### 13.10.1 Lazy writes

A write call (`kernel.write_signal`, `kernel.write_attr`, or any
write inside `kernel.transaction`) records the new value in the
reactive state buffer's back-buffer cell. **No derived recomputation
or recurrent advancement happens at write time.** Writes accumulate
in the back buffer until the next `kernel.publish()`.

Dirty bits are determined at publish time, not per write
(§13.10.2 step 1). This makes value-change semantics correct under
net-revert patterns: a sequence of writes that ends with the cell's
value equal to the previous-published value produces no dirty bit
and fires no triggers — regardless of intermediate values during
the accumulation.

```
// Outside or inside a transaction, identical semantics:
kernel.write_signal(x_id, 1);   // back-buffer cell now 1
kernel.write_signal(x_id, 0);   // back-buffer cell back to 0
kernel.publish();                // x's value equals previous publish — no dirty bit
```

This decouples writes from evaluation. Multiple writes between
publishes batch automatically: only the net change from the
previous publish matters.

#### 13.10.2 Publish

`kernel.publish()` performs the full evaluation-and-visibility
operation on the producer thread:

1. **Compute the dirty set.** For each writable cell (signal,
   attr), compare its back-buffer value to its previously-published
   value. Cells whose values differ are *dirty*; cells whose values
   are identical (including those that were written intermediate
   values but reverted before publish) are *not* dirty. Triggers
   listed in arm trigger lists fire when their referenced cell is
   dirty — value-change semantics (§13.2.4.4) operationalized as
   "current-publish value ≠ previous-publish value." Dirty
   propagation extends to all derived cells transitively dependent
   on dirty roots, and to all recurrents whose triggers fired this
   publish. No new dirty bits are added during the rest of the
   publish cycle.
2. **Compute evaluation order.** Topologically sort the per-publish
   DAG (§13.11.3). Nodes in the DAG are:
    - Dirty derived expressions.
    - Each recurrent **arm** whose triggers fired this publish. A
      multi-arm recurrent (§13.2.4) contributes one DAG node per
      fired arm, not one node per recurrent. The arm's `where` guard
      expression (if present) contributes its own dependency edges
      into the DAG — guard reads are not deferred to evaluation;
      they participate in the topological sort.

   Edges are dependencies; recurrent reads are treated as inputs
   (their previous-committed values), which breaks reactive cycles.
   Reads of deriveds, signals, and attrs follow normal dependency
   edges within this publish. Edges whose gate predicate evaluates
   false do not propagate to destination outputs; see §13.9
   (Conditional Activation) for the full semantics, including the
   gate-open transition rule.
3. **Evaluate in topological order.** For each node in topo order,
   invoke its behavior (per §14.6's ABI). Reads resolve as follows:
    - Signal and attr reads → current values in the back buffer
      (most recent writes since the previous publish).
    - Derived reads → this-publish computed values for deriveds
      evaluated earlier in this step; previous-publish committed
      values for deriveds not in the dirty set.
    - Recurrent reads → previous-committed values, always
      (lockstep — §13.2.4.1).

   Derived behaviors write their results into the back buffer.
   Recurrent arm expression results are held aside (not yet
   visible to in-pass evaluation) until step 4.

   **Arm selection at evaluation.** Multi-arm recurrent evaluation
   proceeds in two stages within the publish cycle:
    1. **Guard evaluation order**: each arm's `where` guard expression
       evaluates in topological order over the per-publish DAG
       (alongside other dirty deriveds). This produces a fired/not-fired
       bit per arm.
    2. **Arm selection order**: among the arms with fired triggers AND
       guards evaluated to true (or no guard), the first one in
       declaration order (per §13.2.4) wins. Only the winning arm's
       `next_expr` evaluates; remaining arms' `next_expr` expressions
       are skipped this publish.
4. **Commit recurrent advancement.** Write the next values
   computed in step 3 into the recurrent cells. After this step,
   recurrent reads return their newly-advanced values.
5. **Atomic swap.** The producer atomically swaps the current
   pointer to the back buffer (§14.3.3.1). Consumers' subsequent
   swaps observe the just-published state.
6. **Clear dirty bits.** Ready for the next publish.

Writes that occur during publish execution are forbidden (single
producer; the producer is busy in the publish call). Writes from
the same thread between publish calls accumulate as usual.

#### 13.10.3 Topological order and tiebreaker

Within a publish cycle, dirty deriveds and recurrent arm
expressions evaluate in topological order over the per-publish DAG.
Topological order ensures that each node's dependencies have stable
values when the node itself is evaluated.

When two nodes are at the same level (neither depends on the
other), the compiler chooses a deterministic tiebreaker:
**source declaration order**. The cell declared earlier in source
evaluates first. Since the two are not dependency-related, the
choice does not affect correctness — but determinism matters for
reproducibility (same program, same inputs, same output trace).

For cells across different node instances at the same level, the
placement order at construction time is the tiebreaker.

Arm expressions across multiple recurrent cells evaluate in
lockstep (§13.2.4.1); no internal ordering between them is
observable, because none of them sees another's just-advanced
value.

#### 13.10.4 Transactions

The host may opt into transactional batching of multiple writes
that should commit as one logical change:

```
kernel.transaction(|tx| {
  tx.write_signal(a_id, new_a);
  tx.write_signal(b_id, new_b);
});
```

Writes within a transaction accumulate in the back buffer and
commit atomically at transaction close. Properties:

- **Panic during the closure:** trap-track semantics of §13.13.1
  apply — the process aborts. There is no rollback; the back-buffer
  state at the moment of abort is irrelevant because the process
  is terminating. Atomicity of grouped writes is trivially
  preserved by process death.
- **Nesting:** nested transactions are flattened — only the
  outermost `kernel.transaction` commits. Inner `kernel.transaction`
  calls are no-ops with respect to commit. All writes since the
  outer transaction's start are committed together at outer close.
- **Cancellation:** an explicit `tx.abort()` method rolls back the
  transaction's accumulated writes. The closure returns normally;
  the back buffer is restored to its pre-transaction state. This
  is the only rollback path.
- **Relationship to publish:** transaction close commits writes to
  the back buffer. Dirty cells remain dirty until the next
  `kernel.publish()`, which performs evaluation and visibility.
  Transactions provide *atomicity of grouped writes*; publish
  provides *evaluation and visibility*.

Outside transactions, individual `kernel.write_*` calls behave as
if each were its own one-write transaction.

### 13.11 Cycle Handling

Cycles in Ductus's reactive graph are handled at two distinct
layers: **reactive expression cycles** between reactive cells
within and across nodes, and **topology cycles** between node
instances via connection placements. Each has its own rules.

#### 13.11.1 The reactive dependency graph

The compiler constructs the reactive dependency graph by walking
every `derived` expression's body and every recurrent arm
expression's body, recording for each the set of reactive cells it
reads. Edges go from each read cell to the reading expression's
output cell. Signal, attr, derived, and recurrent reads all
contribute edges.

The reactive dependency graph is the basis for the per-publish DAG
constructed each publish (§13.10.2 step 2).

#### 13.11.2 Reactive expression cycle rules

**Derived↔derived cycles are forbidden.** A cycle consisting only
of derived-to-derived edges has no temporal delay element. Within
a single publish, derived `a` reading derived `b` while derived
`b` reads derived `a` has no resolution at any single moment.
This is a mathematical impossibility, not a design choice. The
compiler rejects such cycles:

```
error: instantaneous cycle in reactive expressions
  derived `a.x` depends on `b.y`
  derived `b.y` depends on `a.x`
  hint: introduce a `recurrent` declaration on the cycle, or
        eliminate the cyclic dependency
```

**Recurrent self-reference and cross-reference are allowed.** A
recurrent cell's arm expression may read the recurrent's own
previous value (`on t: self.x + 1`) or another recurrent cell's
previous value (`on t: self.other.value`). These do not form
instantaneous cycles because recurrent reads always return the
previous-committed value (lockstep — §13.2.4.1). The per-publish
DAG treats every recurrent read as an input, breaking the static
cycle temporally.

Example (allowed):

```
node Filter:
  attr input: f32 = 0.0
  recurrent previous: f32 = 0.0
    | on sample_clock: self.current
  derived current: f32 =
    0.5 * self.input + 0.5 * self.previous
```

`current` reads `previous`; `previous`'s arm reads `current`.
The static graph has a cycle, but the lockstep semantics make this
well-defined: each publish, `current` reads `previous`'s last-
committed value, then `previous` advances to `current`'s new value
at commit time.

#### 13.11.3 The per-publish DAG

To evaluate a publish, the kernel constructs the *per-publish DAG*
by treating every recurrent read as an *input* — its value is
whatever was committed at the end of the previous publish, not
what will be committed at the end of this publish. This breaks
all valid reactive cycles, producing a DAG.

Reads OF a recurrent cell — from any expression context (derived
bodies, recurrent arm `next_expr` bodies, `where` guards) — always
return the previous-committed value. This is the rule that breaks
reactive cycles.

Reads FROM a `where` guard (its own input cells) are NOT treated as
previous-publish inputs. The guard evaluates within the current
publish to determine whether its arm fires (per §13.10.2 step 2). The
two rules are not in conflict: "reads OF recurrents" refers to what
value a recurrent cell yields when read; "reads FROM a guard" refers
to which cells the guard's expression itself reads.

The per-publish DAG is what gets topologically sorted in §13.10.2
step 2.

#### 13.11.4 Recurrents as delay elements

A recurrent cell on a cycle behaves as a one-publish delay
element: it always reads the previous-committed value, regardless
of what its firing arm computes this publish. The end-of-publish
commit (§13.10.2 step 4) is what advances the cell for the next
publish to observe.

This is the same semantic primitive used by hardware registers
(Verilog `<=` non-blocking assignment), synchronous-dataflow
languages (Lustre `fby`), and signal-flow audio languages
(Faust `~`). The behavior is fully specified at the language
level; the kernel requires no per-implementation convention beyond
the `recurrent` declaration.

#### 13.11.5 Topology cycles

Distinct from reactive expression cycles, a *topology cycle* is a
cycle in the construction-time *topology graph*.

> **Topology graph.** Nodes correspond to placed instances; for
> each placed connection, a directed edge runs from the connection's
> `from`-endpoint instance to its `to`-endpoint instance, labeled
> with the connection type.

A topology cycle is a sequence of distinct directed edges
returning to its starting node.

Example: instance A has a connection to B; B has a connection back
to A. The edges A→B and B→A form a topology cycle.

Topology cycles are governed by the `Circularity` trait (§13.6.5):

> Every topology cycle in the construction-time node graph must
> traverse at least one connection whose type satisfies
> `Circularity`.

The compiler walks the construction-time graph, identifies cycles,
and verifies each cycle has at least one `Circularity`-satisfying
connection. Cycles consisting only of non-`Circularity`
connections are compile errors.

```
error: topology cycle with no Circularity-satisfying connection
  instance `a` connects to `b` via `MyConn`
  instance `b` connects to `a` via `MyConn`
  hint: at least one connection on this cycle must use a
        connection type that satisfies `Circularity`
```

The `Circularity` trait is a marker — its semantic effect is
domain-defined (typically: connections that introduce a temporal
break between source and destination, so cycles through them
cannot loop instantaneously at runtime). Connection types that
imply simultaneous source-destination activation should not
satisfy `Circularity`.

Topology cycles and reactive expression cycles are independent
concerns: a topology cycle can exist with no reactive expression
cycle (the nodes don't read each other's cells), and reactive
expression cycles can exist within a single node with no
involvement of connections. Each cycle layer has its own
validation pass.

### 13.12 The Reactivity Boundary

The reactivity boundary determines which expressions become reactive
and which remain ordinary computation.

#### 13.12.1 Provenance tracking

The compiler computes, for each expression, its *provenance set*:
the set of reactive cells (signals, attrs, recurrents, derived
results) the expression reads, including transitively through
function calls and field accesses. An expression is *reactive* iff
its provenance set is non-empty.

The compiler uses provenance to:

- Decide which cells to include in a derived's dependency set
  (used by the dirty-bit propagation in §13.10.1).
- Diagnose reactivity-where-compile-time-required errors with
  precise blame: *"value of `x` is reactive because it depends on
  signal `mouse_position` at line 14."*
- Reject use of reactive values in positions where compile-time-
  known values are required (§2.4.2).

#### 13.12.2 Functions are reactive-transparent

A function body is not itself reactive. A function takes parameters
as ordinary values and returns ordinary values; it has no knowledge
of signals, attrs, recurrents, or deriveds beyond what its
parameters carry.

Reactivity emerges at the call site, not in the function body. When
a reactive expression calls `some_fn(signal_a, signal_b)`, the
expression's provenance set includes `signal_a` and `signal_b`
(plus the transitive provenance of any reactive reads inside
`some_fn` — see below).

When `signal_a` or `signal_b` changes, the containing reactive
expression becomes dirty and re-evaluates. Re-evaluation re-runs
`some_fn` with the new argument values. The function sees only
the new concrete values; it never observes "the signal."

Reactive transparency applies to `fn` declarations only. Operators
(§13.17) are *not* reactive-transparent — they allocate cells per
instantiation and have distinct call-site semantics. The
distinction at the call site is by callee declaration kind:
`some_fn(my_signal)` evaluates per-emission via reactive
transparency; `some_op(my_signal)` instantiates an operator with
internal state.

Functions may also accept `Signal[T]` parameters (§13.2.8). When a
function declares `fn some_fn(s: Signal[T])`, the parameter binds
to the cell reference itself rather than its current value. This is
distinct from the per-emission behavior described above. The
compiler distinguishes by the declared parameter type; no
call-site syntactic difference. Use cases for `fn(Signal[T])` are
narrow; the typical `fn` declaration uses bare `T` parameters and
relies on reactive transparency at the call site.

##### 13.12.2.1 Transitive provenance through functions

If a function's body reads a reactive cell directly (e.g., reads
a signal declared at module scope), the function's return value
inherits that provenance. Calling such a function from a reactive
expression adds the directly-read cells to the expression's
provenance set.

```
signal global_offset: f32 = 0.0

fn shifted(x: f32) -> f32:
  x + global_offset                    // reads signal `global_offset`

derived adjusted: f32 = shifted(self.base_value)
                       // provenance = { self.base_value, global_offset }
```

The compiler's provenance analysis is transitive — it follows
function calls to find all reactive reads. Module-level globals
read by called functions are included.

##### 13.12.2.2 Conservative branching

When a function's body branches based on its arguments, the
provenance contribution of each branch is computed independently
and unioned. If branch A reads cell X and branch B reads cell Y,
the function contributes {X, Y} to its callers, even though only
one branch executes per call. This is a conservative
over-approximation: cell Y is included in dependency sets even
when the A branch is taken, potentially causing unnecessary
re-evaluation. This is correct (the system never under-tracks
dependencies) and is the standard reactive-runtime treatment.

#### 13.12.3 Closures snapshot reactive values

Per §11.10, closures capture by value (Copy types only). If a
closure is constructed with a reactive value in scope, it captures
the value at construction time as a snapshot — not the live cell.

```
let current_threshold: f32 = some_signal    // snapshot at this moment
let predicate = |x: f32| x > current_threshold
                        // closure captures the snapshotted f32 value, not the signal
```

Calling `predicate` later does *not* observe subsequent changes to
`some_signal`. The closure is not reactive in the sense of
participating in the dependency graph.

To use a value reactively, the user writes a derived expression
that reads the reactive cell directly (or calls a function that
reads it). Closures are for snapshot semantics; derived expressions
are for live reactive semantics.

#### 13.12.4 Reactive cell types and storage

Reactive cells (signal, attr, recurrent, derived values) accept
**any type**. The compiler determines the cell's storage strategy
from the type's size and shape:

**Direct in-cell storage (no indirection):**

- Types whose size is ≤ the platform's atomic word width (typically
  one i64 = 8 bytes) are stored directly in the reactive state
  buffer's cell. The atomic publication (§14.3.3) is a single
  atomic store. No allocation, no refcount, no pool.
- On platforms supporting wider atomics (x86_64 with `CMPXCHG16B`,
  ARM64 with `LDXP`/`STXP`), types of size 9–16 bytes are stored
  across two consecutive 64-bit cells with atomic-pair updates
  (§14.3.4). On platforms without wide-atomic support, types in
  this range fall back to handle-based storage.
- `Result[T, E]` and `Option[T]` follow the same rule: if the total
  bit width (discriminant + maximum payload) fits the atomic word,
  direct storage applies.

**Handle-based pool storage (indirection):**

- Types whose size exceeds the platform's atomic word width are
  stored as handles (one i64 per cell) into a per-type pool. The
  cell stores the handle; the actual value lives in the pool.
- Dynamically-sized types (`string`, `Vec[T]`, `HashMap[K, V]`,
  and other heap-allocated dynamic-size collections) always use
  handle-based storage. `string` uses the existing string pool
  (§14.5); other dynamic types use per-type pools generated by the
  compiler.
- Refcounting on the pool entry manages lifetime: when the cell is
  overwritten and no consumer holds a reference, the pool slot is
  released.

**Pool mechanics:**

- Per-type pools. Each reactive cell type that requires handle-based
  storage has its own pool, sized at kernel construction based on
  graph specification.
- The cell still occupies one i64 slot in the reactive state buffer
  (the handle); the triple-buffer atomic swap publishes the handle
  unchanged.
- Producer cost: when writing a complex-typed cell, the producer
  allocates a pool slot (or reuses one), copies the value in, and
  writes the handle into the back buffer. This involves work
  proportional to the value's size, plus a pool acquire.
- Consumer cost: dereferencing the handle to read the value.

**Performance implications:**

- For real-time domains (audio DSP, animation hot paths), prefer
  reactive cells whose types fit direct storage. Pool storage
  involves allocation and refcounting that may not be acceptable
  inside an audio callback or render loop.
- Direct storage has zero overhead vs. plain atomic reads/writes.
- Handle storage adds an indirection on read and a pool allocation
  on write. Acceptable for cold paths, configuration cells,
  network/IO results, etc.

**Cell-fit examples:**

| Type                                            | Storage                                                    |
|-------------------------------------------------|------------------------------------------------------------|
| `i32`, `f32`, `bool`                            | direct (4 bytes)                                           |
| `i64`, `f64`                                    | direct (8 bytes)                                           |
| `(i32, f32)` tuple                              | direct (8 bytes)                                           |
| `Option[i32]`                                   | direct (1-bit tag + 4 = 5 bytes)                           |
| `Result[i32, i32]`                              | direct (1-bit tag + 4 = 5 bytes)                           |
| Record with two `i64` fields                    | direct on wide-atomic platforms (16 bytes); pool otherwise |
| Record with five `i64` fields                   | pool (40 bytes)                                            |
| `string`                                        | pool (variable; via string pool §14.5)                     |
| `Vec[i32]`                                      | pool (variable)                                            |
| `HashMap[K, V]`                                 | pool (variable)                                            |
| Fixed-size array `T[N]` with N×sizeof(T) ≤ word | direct                                                     |
| Fixed-size array `T[N]` with N×sizeof(T) > word | pool                                                       |

Sizes shown are value widths. Storage is in 64-bit (8-byte) cells:
values ≤8 bytes occupy one cell with appropriate padding/extension;
larger values span multiple consecutive cells per §14.3.2 or use pool
storage per §14.3.5.

**Dynamic-size handle-based types:**

The stdlib provides three dynamic-size collection types usable as
reactive cell types. Each uses handle-based pool storage. The
cost model below is normative — implementations must achieve the
documented complexity bounds.

| Type             | Append/push  | Read by index | Memory pattern              |
|------------------|--------------|---------------|-----------------------------|
| `Vec[T]`         | O(log32 n)   | O(log32 n)    | Persistent trie, structural sharing across versions |
| `SmallVec[T; N]` | O(n) bounded | O(1)          | Inline storage up to N elements, heap beyond        |
| `RingBuf[T; N]`  | O(1)         | O(1)          | Fixed-capacity ring; oldest dropped when full       |

The `T; N` form distinguishes the type parameter T from the
const-generic parameter N. Generic syntax in Ductus uses commas
between type parameters and semicolons before const-generics.

`Vec[T]` is the default for unbounded growth; the persistent
vector trie (Clojure/Scala/Rust `im::Vector` family) provides
sublinear append and read with structural sharing across
triple-buffer versions. `SmallVec[T; N]` optimizes the common
case of small bounded collections with cache-friendly inline
storage. `RingBuf[T; N]` provides constant-time bounded history
with automatic eviction.

The functional builder API preserves the no-mutation rule
(§13.2.7):

- `vec.with(value)` returns a new `Vec[T]` with `value` appended.
- `vec + value` is equivalent (operator form).
- The `recurrent`'s arm expression returns the new value;
  the kernel commits it through triple-buffer rotation.

Implementation strategies (Vec uses persistent trie; SmallVec
uses inline+heap; RingBuf uses fixed ring) are observably
indistinguishable from "always returns new" semantics. Sharing
and in-place optimization are kernel concerns, transparent at
the language level. See §14.3 (extensible pools) for the runtime
mechanism, §14.8 for triple-buffer eviction ordering.

**Cost model for users not using dynamic types:**

If user code never references `Vec[T]`, `SmallVec[T; N]`, or
`RingBuf[T; N]`, the runtime cost is zero. The reactive state
buffer remains a flat fixed-size table; the extensible pool
machinery is not exercised. Binary size grows slightly only when
these types are used. This preserves "pay for what you use" for
hard-realtime workloads.

**Not permitted as reactive cell types in v1:**

- `dyn` trait objects (no fixed in-cell representation; type
  identity not tracked statically).
- Functions and closures (require captured environment management
  beyond what the reactive cell model supports).

For "collection of reactive things" patterns, prefer composition
via parts (§13.4): a parent node with N parts of the same child
type, each part holding its own attrs/deriveds. This is the
canonical reactive composition mechanism. Reactive cells of
collection types (`Vec[T]`, etc.) work via pool storage but each
write involves rebuilding/replacing the collection — fine for
batch updates, less suited for fine-grained mutations.

#### 13.12.5 Reactivity vs compile-time evaluation

A reactive value cannot be used where a compile-time-known value
is required (§2.4.2, §2.4.4). Specifically:

- Array sizes: `i32[some_signal]` is a compile error.
- Const-generic arguments: `Buffer[some_signal]` is a compile
  error if `some_signal` flows into a const-generic position.
- `const` declarations: a `const` whose RHS is reactive is a
  compile error per §2.4.1.2.

The compiler tracks reactivity provenance to provide precise
diagnostics for these cases.

### 13.13 Error Handling in Reactive Contexts

Ductus's two-track failure model (§8.1) applies uniformly to
reactive contexts.

#### 13.13.1 Traps abort the process

A reactive expression — derived expression or recurrent arm
expression — that traps during evaluation, from arithmetic
overflow under default operators (§4.6.1), division by zero, an
out-of-range array index, or explicit `panic`, follows the
trap-track semantics of §4.6.1: the process aborts.

The kernel does not isolate traps within behavior invocations. There
is no "errored cell" sentinel state at the kernel level, no
`catch_unwind` boundary, no continuation past a trap. A trap is a
bug, and bugs end the program.

#### 13.13.2 Recoverable failures via value-track errors

Programs that need to handle recoverable failures use the
value-track error model (§8). Specifically: declare the derived's
type as `Result[T, E]` (or `Option[T]`), have the expression
produce `Err(...)` (or `None`) explicitly for failure cases via
checked arithmetic operators (§4.6.4) or pattern matching, and
propagate through `?` or `match` in downstream expressions.

```
node Divider:
  attr numerator: f32
  attr denominator: f32
  derived quotient: Result[f32, DivideError] =
    if self.denominator is 0.0:
      Err(DivideError::ByZero)
    else:
      Ok(self.numerator / self.denominator)

node Consumer:
  parts: Divider
  derived report: string =
    match self.divider.quotient:
      Ok(value): "result: {value}"
      Err(DivideError::ByZero): "result: undefined"

Consumer my_consumer:
  Divider divider               // names the contained Divider part
```

The divide-by-zero case never traps; it produces `Err(...)`. The
`Consumer.report` derived handles both branches explicitly. No
kernel-level error machinery is involved.

For arithmetic operations that may overflow but should produce
recoverable errors, use the checked variants (`+?`, `-?`, etc.)
per §4.6.4. Their results are `Option[T]` values that flow through
the type system.

#### 13.13.3 The reactive context is not an exception

The reactive evaluation context does not modify Ductus's trap
semantics. A behavior that traps aborts the process, same as a
free function or function-body trap. Authors expecting graceful
handling must use value-track errors; the language does not
provide a hidden recovery mechanism.

### 13.14 Host API

The kernel exposes an API for host code (the application embedding
the kernel) to drive and observe the reactive graph. The shape of
the API is normative; the specific syntax in user-facing code
depends on the host language (Rust, etc.) and is implementation-
defined.

#### 13.14.1 Lifecycle

The kernel's lifecycle proceeds in phases:

**Startup:**

1. Load the graph specification (per §15.4).
2. Allocate the reactive state buffer (per §14.3).
3. Initialize all reactive cells (signals, attrs, recurrents,
   deriveds) and evaluate all `when` predicates in topological order
   over their init-time read dependencies, per §13.2.6's startup
   pass rules. Signal cells receive declared initial values; attr
   cells receive declared defaults (or placement-supplied values);
   recurrent cells receive declared initial values (the arm
   `next_expr` is NOT evaluated at startup); derived cells are
   computed by evaluating their expression bodies; `when` predicates
   are evaluated to determine each instance's initial gate state.
   Placement-level `when` predicates (§13.9.3) are evaluated
   alongside type-level `when:` predicates in the same topological
   pass; placement-level overrides type-level per §13.9.5 with the
   placement's predicate evaluating in its placement scope rather
   than the type's `self` scope. The kernel does not separate this
   work into per-declaration-kind phases; the topological sort
   determines the order.
4. Perform the first publish (atomic current-pointer swap per
   §14.3.3.1). Consumers' subsequent swaps return real data.

The kernel is "constructing" through steps 1–3; "live" after step
4 completes. Consumer reads via swap before step 4 return a
sentinel (or block, per implementation choice).

**Steady-state operation:**

- Host calls `kernel.write_signal(...)`, `kernel.write_attr(...)`,
  or `kernel.transaction(...)` to update reactive state. Writes
  mark dirty bits; no evaluation runs.
- Host calls `kernel.publish()` to evaluate dirty cells, advance
  recurrent cells per their firing arm expressions, and atomically
  swap the back buffer for consumer visibility.
- Consumer threads call `kernel.swap(...)` to obtain the latest
  published state and read cell values.

**Shutdown:**

1. Stop accepting new signal/attr writes.
2. Drain any in-flight publish (the current publish, if running,
   completes).
3. Drop reactive cells in reverse-of-construction order: connections
   drop before their endpoint instances; within each instance,
   attrs, recurrents, and deriveds drop in reverse declaration
   order (per §14.8 Drop rules).
4. Drop top-level signals.
5. Drop string pool entries (per §14.5).
6. Deallocate the reactive state buffer.
7. Kernel is terminated. Subsequent consumer swaps return a sentinel.

#### 13.14.2 `kernel.write_signal`

A single overloaded call, dispatched by arity:

```
kernel.write_signal(signal_id, value)                      // module-level signal
kernel.write_signal(instance_id, signal_id, value)         // per-instance signal
```

Both arities write a new value to the named signal's cell. The
calls are synchronous and inexpensive: they update the back
buffer's cell and set the dirty bit for dependents. No evaluation
runs at this point.

**Module-level form** — `kernel.write_signal(signal_id, value)`:
writes to a top-level signal. The `signal_id` identifies a
module-scope signal declared per §13.2.1. One cell exists for the
entire program.

**Per-instance form** —
`kernel.write_signal(instance_id, signal_id, value)`:
writes to a node-level or connection-level signal on a specific
instance. The `instance_id` identifies the instance (assigned at
compile time per placement); `signal_id` identifies the signal on
that instance's type. Each placement creates its own cell; the
write targets one specific cell.

Both arities must be called from the producer thread (the kernel's
designated thread for write/evaluation/publish operations; see
§14.7). Other threads write indirectly by enqueueing requests for
the producer thread to apply — that's a host-application concern,
not a kernel concern.

Signal IDs and instance IDs are obtained at compile time from the
graph specification (each signal and each placement has a stable ID
assigned during compilation, per §15.4).

#### 13.14.3 `kernel.write_attr`

```
kernel.write_attr(instance_id, attr_id, value)
```

Writes a new value to the cell of a specific instance's attr.
Otherwise behaves identically to the per-instance form of `kernel.write_signal`:
synchronous, back-buffer-only, dirty-bit propagation, no evaluation.

`instance_id` identifies the instance (assigned at compile time per
placement); `attr_id` identifies the attr on that instance's type.

The same call applies to attrs declared on node instances or
connection instances — both kinds of instance live in the same ID
space.

#### 13.14.4 `kernel.publish`

```
kernel.publish()
```

Performs the complete publish operation specified in §13.10.2:
evaluates dirty deriveds and recurrent arm expressions in
topological order, commits recurrent advancements, and atomically
swaps the back buffer pointer (§14.3.3.1) so consumers see the new
state.

Synchronous; runs on the producer thread; blocks until the publish
completes. Cost is bounded by the size of the dirty set
(deriveds and recurrents with fired triggers) plus the constant
cost of the atomic swap.

Consumer threads see the new state on their next swap. Calling
publish with no dirty cells is idempotent — the buffer swap still
occurs but consumers observe identical state.

The host chooses the publish cadence per its domain: audio hosts
may publish per audio block; UI hosts may publish per frame;
event-driven hosts may publish per event. The kernel imposes no
cadence.

#### 13.14.5 `kernel.transaction`

```
kernel.transaction(|tx| {
  tx.write_signal(a_id, new_a);
  tx.write_signal(b_id, new_b);
})
```

Provides atomic grouping of writes.

The transaction's closure executes synchronously. Writes accumulate
in the back buffer and commit atomically at closure completion. The
full semantic rules for atomicity, panic-on-abort, nesting flattening,
and `tx.abort()` rollback are specified in §13.10.4; the API surface
here is the syntactic invocation form. Transactions provide
*atomicity of grouped writes*; dirty cells remain dirty until the
next `kernel.publish()`, which performs evaluation and consumer
visibility.

#### 13.14.6 `kernel.swap`

```
kernel.swap() -> BufferView
```

Called by a consumer thread to obtain a view of the latest
published state. The call is wait-free: a single atomic load of
the current-pointer per §14.3.3.2.

The returned view provides cell-read access. Reading a cell from
the view is wait-free: a single atomic load. The view remains
valid until the consumer next calls swap; subsequent calls obtain
a new view (potentially pointing at a different buffer if the
producer has published in the interim).

Consumers may hold multiple views concurrently if needed; the
triple-buffer arrangement allows the producer to continue
publishing without disturbing held views.

#### 13.14.7 `kernel.register_reconciler`

```
kernel.register_reconciler(effect_type_name, reconciler)
```

Registers a host-side reconciler for a specific effect type (§13.19).
Must be called before the kernel transitions to the live state
(§13.14.1 step 4); registrations after the kernel is live are
rejected.

The `effect_type_name` is a string identifier matching the name of
an `effect` declaration in the loaded source. The `reconciler` is a
host-language construct (Rust struct, function table, or analogous)
implementing the reconciler interface:

- A *create* hook invoked when a new effect instance enters the live
  graph. Receives the instance ID, current parameter values, and
  initial `desired:` cell values. Returns an opaque reconciler-side
  instance state (closing over external resources as needed).
- An *update* hook invoked when any parameter or `desired:` cell of
  an existing instance becomes dirty. Receives the instance ID, the
  reconciler-side instance state, and current values for parameters
  and desired cells. Writes the alignment outcome into observed cells
  via `kernel.write_signal` (§13.14.2) or `kernel.push_stream`
  (§13.14.8).
- A *teardown* hook invoked when an effect instance leaves scope.
  Receives the instance ID and the reconciler-side instance state.
  Releases external resources.

The host language's binding to this API is implementation-defined.
The normative requirement is that the three hooks be invokable by
the kernel at the publish-cycle boundary, with the per-instance
state managed by the host between invocations.

**Generic effects.** For a generic effect (§13.19.9), reconciler
registration is per-effect-type-per-concrete-instantiation. The
`effect_type_name` includes a mangled suffix encoding the resolved
type parameters; the host registers one reconciler per concrete
instantiation it intends to support. Instantiations without a
registered reconciler are detected at startup (per §13.14.1) and
cause the kernel to refuse the live transition.

**Unregistered effect types.** If the loaded source declares an
effect type but no reconciler is registered, startup fails with a
diagnostic naming the effect type. The kernel does not enter the
live state.

#### 13.14.8 `kernel.push_stream`

```
kernel.push_stream(instance_id, stream_id, value)             // per-instance stream cell
kernel.push_stream(stream_id, value)                          // module-level stream cell
```

Pushes a value into a stream cell. Used by host-side reconcilers
writing into effect `observed:` stream cells, and by host code
producing into top-level stream declarations whose source is host-
defined.

Both arities push to the named stream's ring buffer per the stream's
declared policy:

- For `ring` streams, the push always succeeds; if the buffer is
  full, the oldest unconsumed event is overwritten. The stream's
  `dropped_total` cell increments (§13.18.6).
- For `gate` streams, the push succeeds if a slot is free; otherwise
  the push fails. The stream's `rejected_total` cell increments
  (§13.18.6). The call returns a `bool` indicating success/failure;
  the host decides how to handle rejection.

The push is dirty-tracked: consumers of the stream become dirty and
will re-observe on the next publish. Within a single push, the
event is appended to the back-buffer's ring; the swap on the next
publish makes it visible.

**Per-instance form** —
`kernel.push_stream(instance_id, stream_id, value)` writes to a
stream cell scoped to a specific effect or node instance. The
`instance_id` identifies the instance (assigned at compile time per
placement); `stream_id` identifies the stream cell on that instance's
type.

**Module-level form** — `kernel.push_stream(stream_id, value)` writes
to a top-level stream cell. The `stream_id` identifies a module-
scope stream declared per §13.18.

#### 13.14.9 Reconciler protocol

The reconciler protocol is the contract between the kernel and host-
registered reconcilers (§13.14.7). The normative shape of the
protocol:

**Lifecycle alignment with effect instances.**

The kernel maintains a one-to-one correspondence between live effect
instances and reconciler-side instance states. The reconciler's
`create` hook fires when an instance enters scope; `teardown` fires
when it leaves scope; `update` fires when parameters or `desired:`
cells become dirty during a publish.

**Hook timing.**

Hooks fire at the publish-cycle boundary, after publish-and-swap
completes (§13.10.2) and before the next publish begins. The hooks
run on the kernel's producer thread; the reconciler implementation
must not block long-running operations on this thread (long
operations should be dispatched to host-managed worker threads,
with results written back via the host API on completion).

**Write semantics.**

Writes from reconcilers into observed cells via
`kernel.write_signal` and `kernel.push_stream` are dirty-tracked in
the standard way. The writes accumulate in the back buffer; they
become visible to program-side consumers on the next publish.

**Idempotence requirement.**

Reconciler implementations are expected to be idempotent in the
reconciliation sense: applying the same desired state twice produces
the same alignment outcome. The kernel may invoke `update` with
unchanged desired values if a publish marks an instance dirty for
unrelated reasons; reconcilers must not produce duplicate
side effects in this case. The host-side state managed by the
reconciler is the canonical source of "what we've already done";
desired cells describe "what we want to be true."

**Error handling.**

Reconciler errors (network failures, resource exhaustion, host-
level issues) are reported to the program through the effect's
`observed:` cells, typically an `error: Signal[Option[E]]` cell.
Reconcilers do not panic the kernel; reconciler-internal errors are
domain errors expressed through the value track (§8).

A reconciler that panics is treated as a host bug; the kernel's
behavior in that case is implementation-defined (typically the
panic propagates to the host's thread, with the kernel marking the
effect instance as failed and writing an error sentinel to the
instance's observed error cell if such a cell is declared).

### 13.15 Hot Reload of the Reactive Graph

The kernel supports hot reload of the reactive graph when the host
provides updated source code (per §14.9). The reactive system's
specific hot reload semantics are as follows.

#### 13.15.1 Compile-time validation gate

Before any kernel-side action occurs, the new source must compile
under the full Ductus type system (§§1–12) and reactive system
rules (§13). If compilation fails — for any reason, including
dangling references to nodes removed in the new source — the hot
reload is rejected. The kernel continues running the previously-
loaded version, unaffected.

This ensures the kernel never enters a state where compiled
behaviors reference cells that no longer exist or have changed
type.

#### 13.15.2 Cell identity across reloads

Reactive cells are identified across reloads by their *fully-
qualified declaration path*: the dotted sequence of module path,
instance name, and attribute/recurrent/signal/derived name. For
example, `audio.synth_a.osc_1.frequency`. The wire-format syntax for
declaration paths is specified in §15.4.1.1.

For anonymous or duplicated sibling placements (rare; the language
encourages explicit naming per §13.8), the compiler appends an
ordinal suffix `:N` where N is the declaration-order index among
siblings of the same type at the same nesting depth (zero-based).

When a cell with the same fully-qualified path exists in both old
and new source AND has the same type, it is treated as the *same
cell*. Its value is preserved across reload.

When a cell exists in old but not in new, it is a *removal* — the
cell is dropped during reload.

When a cell exists in new but not in old, it is an *addition* — a
new cell is allocated and initialized per the new source's
declared initial value or default.

When a cell exists in both but with different type, it is treated
as removal of the old + addition of the new.

#### 13.15.3 Reload sequence

The kernel performs the reload atomically on the producer thread,
in the following order:

1. Compile new source. On failure, reject reload; kernel state
   unchanged.
2. Acquire a reload lock. Pause acceptance of new signal/attr
   writes from host code (host requests queue).
3. Let any in-flight publish complete; ensure the kernel is in a
   between-publishes state.
4. Compute the diff between old and new graphs: which cells are
   surviving (same path, same type), which are added, which are
   removed.
5. For added cells: allocate space in the reactive state buffer
   and initialize per the new source.
6. For removed cells: invoke their Drop per §14.8, in
   reverse-declaration order. Connections drop before endpoint
   instances; within each instance, attrs, recurrents, and
   deriveds drop in reverse declaration order.
7. Update the behavior table (§14.6.4): register behaviors with
   new content-addressed IDs; deregister behaviors no longer
   present. Behaviors with unchanged content-addressed IDs are
   carried over.
8. Run a re-initialization evaluation pass: for each derived
   whose behavior body changed (different content-addressed ID
   from old to new), recompute its initial value from current
   inputs. For deriveds whose body is unchanged, the value
   persists.
9. Publish the reloaded state (atomic current-pointer swap).
10. Release the reload lock. Resume signal/attr writes; apply any
    queued writes to the new state.

Changes to `when` predicates (added, removed, or modified — §13.9)
are reload-safe. The predicate is structural metadata, not cell
identity; the new predicate participates in the next publish, and
cells retain their values across the reload. The new predicate
takes effect prospectively — historical gate state is not
recomputed.

#### 13.15.4 Constraints on reloadability

Some changes are not safely hot-reloadable in place and require a
restart — either full-kernel or per-instance, depending on the change:

- Changes to the layout of the reactive state buffer that would
  require relocating live cells. The reload's diff-and-apply
  approach handles incremental changes but not whole-buffer
  reorganization. **Full-kernel restart required.**
- Operator-specific changes that require restart for the affected
  operator instances:
    - Operator signature changes (parameters added, removed, or
      retyped; return type changed).
    - Internal cell type changes within an operator body.
    - Changes to a cell's `= initial` expression in a way that would
      alter its current state semantics.

  See §13.17.10 for full operator-reload rules. **Per-instance
  restart** suffices: the affected operator instances are
  recreated; the rest of the kernel continues without restart.
- Stream-specific changes that require restart for the affected
  stream cells (and their consumers):
    - Element type changes (incompatible structural change to `T`).
    - Policy changes (`ring` ↔ `gate`).
    - Capacity changes.

  See §13.18.11 for full stream-reload rules. **Per-instance
  restart** suffices.
- Effect-specific changes that require restart for the affected
  effect instances:
    - Effect parameter signature changes (added, removed, retyped,
      reordered parameters).
    - `desired:` or `observed:` cell type changes.
    - Stream/Sink cell policy or capacity changes within an effect.

  See §13.19.11 for full effect-reload rules. **Per-instance
  restart** suffices; the host's reconciler receives a teardown
  call for the affected instances.

Implementations detect these cases during the diff phase and either
reject the reload or schedule the appropriate restart (full-kernel
or per-instance). The kernel diagnoses which class of change
occurred.

#### 13.15.5 Hot reload of streams

Stream cell identity across reloads follows the same fully-qualified
declaration path rule as other reactive cells (§13.15.2). Two
additional rules apply specific to streams:

**Buffer preservation rule.** A stream's ring buffer is preserved
across reload iff the stream's *type signature* is byte-identical
between old and new code:

- Element type `T` (structurally identical, not just same-named).
- Policy (`ring` or `gate`).
- Capacity (the integer literal `N`).

When all three match, the buffer's contents survive. When any
differs, the buffer is reset to empty and all consumer cursors
restart at the empty position.

**Source expression changes are reload-safe.** The `= source`
portion of a stream declaration may change freely without affecting
buffer preservation. Producers that change their emission logic
across reload continue to push into the same buffer; existing
buffered events from prior emissions remain available to consumers
until overwritten by the policy.

**`@reset_on_reload` annotation.** A stream declaration carrying the
`@reset_on_reload` annotation always resets its buffer across
reload, regardless of type-signature match:

```
@reset_on_reload
stream ring[1024] events: LogEntry = source
```

This is appropriate when buffered events from the prior program
version would be misinterpreted by the new version's consumers.

**Cursor identity across reload.** A consumer's cursor is preserved
when the consuming operator (or derived) instance is preserved per
its own identity rule (§13.17.10). A new consumer added in the new
source starts at the current head — it observes only events arriving
after the reload. A removed consumer's cursor is dropped.

**Observation cell continuity.** The synthesized observation cells
(`pending_count`, `pressure`, `is_full`, `dropped_total`,
`rejected_total`, `last_overflow_at` — see §13.18.6) survive reload
along with the buffer when type signature matches; they reset when
the buffer resets.

#### 13.15.6 Hot reload of effects

Effect instance identity across reloads follows the operator
identity rule (§13.17.10): the instance is identified by enclosing
scope, effect type name, and argument bindings, with tolerance for
positional moves within the same scope.

**Cell preservation within preserved instances.** When an effect
instance is preserved across reload, the cells declared in its
`desired:` and `observed:` blocks follow per-cell preservation rules:

- `Signal[T]` cells: preserved when type matches; reset to initial
  value if type changes (per §13.15.2).
- `Stream[T]` and `Sink[T]` cells: preserved per stream reload rules
  (§13.15.5).

**Reload-safe effect changes** preserve the instance and most cells:

- Adding a new cell to `desired:` or `observed:` — the new cell is
  initialized fresh.
- Changing the initial-value expression of a `desired:` or
  `observed:` Signal cell — existing committed values persist;
  the new initial-value expression applies only to fresh instances.
- Changing a parameter-derived `desired:` cell's derivation
  expression — the cell re-evaluates on the next publish with the
  new logic.
- Changing the visibility of the effect, the generic-parameter
  bounds, or other declaration-level metadata that does not affect
  cell shape or parameter signature.

**Reload-unsafe effect changes** require per-instance restart per
§13.15.4:

- Parameter signature changes.
- Cell type changes in `desired:` or `observed:`.
- Stream/Sink policy or capacity changes.

When per-instance restart fires for an effect instance, the kernel
invokes the host's reconciler teardown hook (§13.14.9), allowing the
host to release external resources, and then constructs the new
instance under the new declaration. The reconciler's create hook
fires for the new instance.

**Effect type identity.** When an effect's declared name changes
(e.g., `effect fetch` becomes `effect cached_fetch`), the kernel
treats this as removal of the old effect type and addition of a new
one. Instances of the old type are torn down; instances of the new
type (if any) are constructed fresh. The host must register a
reconciler for the new effect type via `kernel.register_reconciler`
before the reload reaches the live state.

### 13.16 Interaction with the Implementation (§14)

§13 specifies the reactive system's source-level semantics; §14
specifies the implementation model. Cross-references:

- Reactive cells (signal, attr, recurrent, derived) live in the
  triple-buffered reactive state buffer per §14.3. Single-cell
  types (per §13.12.4) map to single AtomicI64 cells.
- Stream cells (§13.18) allocate ring buffers from per-`(T, N)`
  pools per §14.3.5; their metadata (head pointer, observation
  cells) lives in the standard triple-buffered area per §14.4.
- Effect instances (§13.19) are groupings of standard reactive
  cells (signal, stream, sink) plus host-side reconciler state.
  No new storage category per §14.4; per-instance state in the
  reconciler is managed by the host outside the kernel's buffer.
- The producer role per §14.7 is the kernel's reactive evaluation
  thread. It applies host writes to the back buffer, runs publish
  cycles (recurrent arm evaluation, derived behavior
  invocations, atomic swap). In typical deployments, the host's
  main thread plays the producer role; in other deployments, a
  kernel-configured thread does.
- The consumer role per §14.7 is any thread reading published
  state via swap. Consumer threads do not invoke behaviors; they
  read the results of past publishes.
- Behaviors invoked during reactive evaluation — both derived
  expressions and recurrent arm expressions — conform to the
  ABI of §14.6: a uniform `fn(kernel: &KernelHandle, instance:
  InstanceId) -> ()` signature, with stateless semantics and
  content-addressed identity (§14.6.4).
- Host-registered reconcilers (§13.19.14) are dispatched at the
  publish boundary via the host API (§13.14.7, §13.14.9). They
  run on the kernel's producer thread; long-running operations
  are dispatched to host-managed worker threads with results
  written back via the host API on completion.
- The graph specification (§15.4) carries the structural information
  the kernel needs to construct the reactive state buffer, build
  dependency edges, distinguish attr cells from recurrent cells,
  enumerate stream cells and effect instances, and dispatch
  behaviors.
- Hot reload at the source level (§13.15, including stream and
  effect reload in §13.15.5–§13.15.6) maps to the §14.9
  mechanism: the kernel diffs behaviors and cells between old
  and new compiled output, applies the diff atomically, and
  publishes.

### 13.17 Operators

An *operator* is a reusable, cell-allocating reactive transformation
declared with the `operator` keyword. Operators take `Signal[T]`
inputs (and optionally non-reactive value parameters), allocate
internal reactive cells (recurrents and/or deriveds) per
instantiation, and produce a `Signal[T]` output. They are the
primary mechanism for composing reactive transformations.

Operators are distinct from `fn` declarations:

- `fn` is reactive-transparent (§13.12.2). It takes value
  parameters, returns values, and allocates no cells.
- `operator` is *not* reactive-transparent. It takes cell
  references (`Signal[T]`), allocates internal cells per
  instantiation, and is wired into the reactive graph at the call
  site.

#### 13.17.1 Concept

Stateless operators wrap a pure projection over a source cell:

```
operator double(source: Signal[f32]) -> Signal[f32]:
  source * 2
```

Stateful operators allocate recurrent state per instantiation:

```
operator smooth(source: Signal[f32], rate: f32 = 0.1, clock: Signal[u64]) -> Signal[f32]:
  recurrent state: f32 = source
    | on clock: state + (source - state) * rate
  state
```

Each instantiation of a stateful operator creates fresh internal
cells; multiple call sites do not share state.

#### 13.17.2 Declaration

```
operator name[GenericParams]?(params...) -> Signal[T]:
  body
```

- `name` is a snake_case identifier.
- `GenericParams` are optional type parameters with optional trait
  bounds (§3, §5).
- `params` is a comma-separated parameter list (§13.17.3).
- The return type is always `Signal[T]` for some value type `T`.
- The body is a sequence of reactive declarations (recurrents,
  deriveds) followed by a final expression that becomes the output.

Operators may carry visibility modifiers (`public`, `shared`,
`private`) per §10.

#### 13.17.3 Parameters

Operator parameters are of two kinds, distinguished by declared
type:

**Cell-bound parameters** (`name: Signal[T]`):
- Bind to a reactive cell at instantiation.
- The operator tracks the cell's changes over time via the reactive
  engine.
- Inside the body, the parameter is treated as a cell of value type
  `T` — references read the cell's current value.

**Value parameters** (`name: T`):
- Snapshot at instantiation. The value is fixed for the lifetime of
  this operator instance.
- Inside the body, the parameter is a compile-time-fixed value.
- Useful for configuration that does not change: smoothing rates,
  thresholds, modes, etc.

The author chooses for each parameter based on intent. A parameter
the user expects to vary at runtime (e.g., a UI knob driving a
smoothing rate) should be `Signal[T]`; a parameter that is a
deployment-time choice should be `T`.

```
operator smooth(
  source: Signal[f32],         // cell-bound: tracked over time
  rate: f32 = 0.1,             // value: snapshotted at instantiation
  clock: Signal[u64],          // cell-bound: drives the trigger
) -> Signal[f32]:
  ...
```

**Default values** are allowed on value parameters. Default values
on `Signal[T]` parameters are not allowed in v1 (a default cell
reference has no clear meaning; if needed, use a stdlib helper
that constructs a constant cell).

**Default-parameter ordering.** Defaulted-before-non-defaulted ordering
follows the general rule in §3.5.4 (which applies uniformly to
functions, operators, and constructors); operators have no special-case
relaxation.

**At call sites:**

- Literals passed to `Signal[T]` parameters are wrapped as implicit
  constant signal cells (compile-time-fixed `Signal[T]` values). Cost:
  one cell per literal at the call site (effectively zero — folded by
  the compiler when possible).
- Cells passed to `Signal[T]` parameters bind directly.
- Values passed to `T` parameters are evaluated and snapshotted.

##### 13.17.3.1 Signal[T] auto-deref in expression contexts

When an expression context requires a value of type `T` and the
supplied expression has type `Signal[T]`, the compiler implicitly
inserts a read of the cell — `signal` is dereferenced to its current
value. The provenance tracking (§13.12.1) records the cell read as a
dependency, so the surrounding expression becomes reactive on changes
to that cell.

```
operator example(s: Signal[f32]) -> Signal[f32]:
  derived doubled: f32 = s * 2.0       // s: Signal[f32] auto-derefs to f32 in arithmetic context
  doubled
```

The implicit deref applies wherever a `Signal[T]` flows into a
position expecting `T`: arithmetic operands, function-call arguments
typed `T`, attribute initial-value expressions, derived bodies,
recurrent arm expressions. It does NOT apply when the context expects
`Signal[T]` directly (operator parameters, function parameters typed
`Signal[T]`, pipe-form `|>` LHS) — in those cases the cell reference
is bound without dereferencing.

The auto-deref is a compile-time mechanism; no runtime cost beyond
the cell read itself.

#### 13.17.4 Body

The operator body is a sequence of reactive declarations followed
by a final expression. Permitted body items:

- `recurrent` declarations (with all extensions per §13.2.4).
- `derived` declarations.
- `let` bindings for intermediate values, including
  runtime-evaluated reads of `Signal[T]` parameters and other
  cells in scope. A `let` binding's right-hand side is evaluated
  in a reactive context — reads of cell-bound parameters return
  their current values, and the binding's value is recomputed
  whenever any read cell changes (the binding behaves as a
  synthesized derived for dependency-tracking purposes).
- The final expression — the value (or cell) returned as the
  operator's output.

Not permitted in operator bodies:

- `signal` declarations. Operator-internal cells cannot be
  host-writable; the host has no addressing mechanism for cells
  inside an operator instance.
- `attr` declarations. Per-instance configuration is expressed via
  parameters, not internal attrs.
- Side-effecting statements. The body is reactive — declarative,
  not imperative.

The final expression's type must be either `T` or `Signal[T]`
(matching the operator's return type `Signal[T]`). If the type is
`T`, the compiler synthesizes a derived cell holding the final
expression's value, and exposes that cell as the operator instance's
output. If the type is already `Signal[T]` (e.g., a named recurrent
or derived in the body), that cell is the output directly — no
synthesis needed.

#### 13.17.5 Output

Every operator returns a single reactive cell. The return type is
typically `Signal[T]` for value-shaped operators, but may be any
`Cell[T]` per §13.18.5 — including `Stream[T]` for event-producing
operators (e.g., `to_stream`, `filter`, `merge`) and `Sink[T]` for
operators that expose write-end stream handles.

```
type PeakResult:
  peak: f32
  count: u32

operator peak_detector(source: Signal[f32]) -> Signal[PeakResult]:
  recurrent (peak, count): (f32, u32) = (source, 0)
    | on source: (
        max(peak, source),
        if source > peak then count + 1 else count,
      )

  PeakResult(peak: peak, count: count)

operator changes[T](source: Signal[T]) -> Stream[T]:
  // emits an event each time source changes
  ...
```

For multiple outputs, return a record or tuple containing reactive
cells:

Consumers project fields via reactive expressions:

```
let result = source |> peak_detector            // Signal[PeakResult]
derived just_peak: f32 = result.peak            // reactive projection
derived just_count: u32 = result.count
```

Field-level reactivity is coarse-grained: a change to any field
invalidates consumers of all fields. For finer granularity,
project early into stable derived cells, or expose distinct
cells from the source. (See §13.2.8 for details on `Signal[T]`
field access.)

#### 13.17.6 Instantiation

Two equivalent call-site syntaxes:

**Function-call form:**

```
let smoothed = smooth(source_cell, rate: 0.1, clock: tick)
```

**Pipe form:**

```
let smoothed = source_cell |> smooth(rate: 0.1, clock: tick)
```

In the pipe form, the LHS of `|>` is bound to the operator's first
positional parameter. The remaining arguments are passed as in the
function-call form. The two forms are observationally identical.

The pipe form is convenient for chaining:

```
derived bar: f32 = 0.0 |> clamp(min: 0.0, max: 1.0) |> smooth(rate: 0.1, clock: tick) |> ease_in_out
```

Each `|>` step is an operator application. The result of each step
is a `Signal[T]` consumed by the next.

**Convention:** the first positional parameter of any operator is
the implicit pipe target. Library authors place the upstream cell
first by convention. For binary operators (those needing two
upstream cells), the first is the pipe target; subsequent cells
are passed by name:

```
operator combine(primary: Signal[T], other: Signal[T], weight: f32) -> Signal[T]:
  ...

// Call:
let result = source |> combine(other: another_signal, weight: 0.5)
```

**Each instantiation allocates fresh internal cells.** Two call
sites of the same operator do not share state. The compiler emits
one allocation set per call site at compile time; the reactive
state buffer reserves space for each instance's internal cells.

##### 13.17.6.1 Operator instance identity

An operator instance is identified by its enclosing scope, the
operator name, and its argument bindings. Two `|>` chains in
different scopes (different modules, different node bodies,
different placements) produce distinct instances with independent
state.

Operator instances do not have user-assignable names. Assigning an
operator's output to a `let` binding names the *output cell*, not
the instance. For reload-identity purposes (§13.17.10), the same
identity scheme is used, with tolerance for positional moves within
the same scope; the binding name has no role.

##### 13.17.6.2 Graph specification

Operator instances contribute to the kernel's graph specification
(§15.4) the same way node placements and connection placements do.
Each instance's internal cells (recurrents, deriveds, synthesized
cells from the operator body and from `let` bindings) are counted
against the reactive state buffer's allocation and against any
per-type pool sizing for dynamic-size cells (§14.3.5).

The compiler enumerates operator call sites at compile time;
recursion through operators is forbidden (an operator may not
transitively instantiate itself), so the static count of operator
instances is bounded and known.

#### 13.17.7 The `|>` operator

`|>` is the pipe-application token. It expresses a connection from
a source cell on the left to a destination on the right. The kind
of connection — apply-and-bind or forward-into — is determined by
the RHS's kind.

**Three dispatch cases**, distinguished by the RHS:

**Case 1: RHS is an operator call** (§13.17). The operator is
instantiated; the LHS is bound to its first positional parameter.
The result is the operator's declared output cell.

```
let smoothed: Signal[f32] = source |> smooth(rate: 0.1, clock: tick)
```

**Case 2: RHS is an effect call** (§13.19). The effect is
instantiated; the LHS is bound to its first positional parameter.
The result is the effect instance value (a composite accessed per
§13.19.7).

```
let f = current_url |> fetch
```

**Case 3: RHS is a `Sink[T]`** (§13.18.4). The LHS must be a
`Stream[T]` of matching element type. A forwarding subscription is
established: each event observed from the source stream is pushed
into the sink. The expression's value is the unit type `()`; the
forwarding subscription lives as long as the enclosing scope.

```
let ws = current_url |> websocket
messages_to_send |> ws.outbound       // forwards stream into the sink
```

LHS rules common to all cases:

- LHS must be an expression of a reactive cell type (`Signal[T]`,
  `Stream[T]`, `Sink[T]`, or any other `Cell[T]` per §13.18.5), or
  an expression convertible to one. Literals are wrapped as implicit
  constant signal cells automatically.
- For Case 3 specifically, the LHS's concrete type must be a
  `Stream[T]` of matching element type. Piping a `Signal[T]` into a
  sink is a type error; use `to_stream` (§13.18.7) to convert first
  if event semantics are desired.

**Precedence:** `|>` is low-precedence, left-associative. Most
arithmetic and logical operators bind tighter. Specifically:

```
a + b |> op            // parses as (a + b) |> op
a |> op1 |> op2        // parses as (a |> op1) |> op2
```

Bitwise `|` (§4.4.2) has the same low precedence as `|>`. Expressions
mixing bitwise OR with pipe application may need parentheses for the
desired grouping.

**`|>` applicability:** `|>` may only apply operators or effects.
Using `|>` with a `fn` is a compile error:

```
let bar = 0.0 |> some_fn       // ✗ error: `|>` requires an operator or effect
```

Diagnostic class:
```
error: `|>` requires an operator or effect on the right-hand side
  --> let bar = 0.0 |> some_fn
                     ^^^^^^^^ `some_fn` is a `fn`, not an operator or effect
  hint: use function call syntax: `some_fn(0.0)`
```

For applying pure functions to reactive cells, use a derived:

```
derived bar: f32 = some_fn(source_cell)
```

Or wrap the function in an operator:

```
operator some_op(source: Signal[f32]) -> Signal[f32]:
  some_fn(source)
```

#### 13.17.8 Generic operators

Operators may take type parameters with optional trait bounds:

```
operator passthrough[T](source: Signal[T]) -> Signal[T]:
  source

operator scan[T: Add + Copy](source: Signal[T]) -> Signal[T]:
  recurrent acc: T = source
    | on source: acc + source
  acc
```

Standard generics machinery applies (§3 traits, §2.2 inference).
Type parameters are resolved at the call site from argument types;
explicit instantiation is supported via turbofish syntax where
inference is ambiguous.

#### 13.17.9 Visibility

Operators carry the standard three-level visibility (§10): `public`,
`shared` (default), `private`. Module-private operators are not
reachable from other modules; public operators may be re-exported.

```
public operator smooth(source: Signal[f32], rate: f32 = 0.1, clock: Signal[u64]) -> Signal[f32]:
  ...

private operator internal_helper(source: Signal[i32]) -> Signal[i32]:
  ...
```

#### 13.17.10 Hot reload of operators

An operator instance is a scoped reload boundary. Within an
instance, the cell-identity rules of §13.15.3 apply: each internal
cell is identified by its declared name and type within the
operator body.

**Reload-safe changes:**

- Changes to the body of recurrent arm expressions, `where`
  guards, or final-expression bodies — same as plain
  recurrent/derived reload safety.
- Adding a new internal cell — new cells are initialized fresh.

**Reload-unsafe changes** are handled per §13.15.4: operator-specific
cases (signature changes, internal cell type changes) trigger
per-instance restart — only the affected operator instances are
recreated, not the whole kernel. Other reload-unsafe changes
(buffer-layout relocation per §13.15.4) require full-kernel restart.

The reload-unsafe operator changes are:

- Operator signature changes (parameters added, removed, or
  retyped; return type changed) — per-instance restart.
- Internal cell type changes — per-instance restart.
- Changes to a cell's `= initial` expression in a way that would
  alter its current state semantics — per-instance restart.

**Call-site changes:**

If a call site changes which operator is invoked (`source |> op_a`
becomes `source |> op_b`), the old instance's cells are dropped
per §14.8 eviction; the new instance's cells initialize fresh.
The two operators are treated as distinct instances even if
op_b's signature matches op_a's.

**Call-site moves:**

If a call site moves within source (e.g., reformatting that shifts
its line/column position) but the operator, its arguments, and its
enclosing scope remain identical, the kernel attempts to preserve
instance identity. The reload's diff phase identifies operator
instances by *(enclosing scope, operator name, argument bindings)*
rather than raw line/column. A pure positional move within the same
scope preserves state; a move to a different enclosing scope is
treated as call-site change (state lost).

If two operator call sites in the same scope cannot be
distinguished by (operator name, arguments) — e.g., two identical
calls `source |> smooth(rate: 0.1, clock: tick)` in the same node
body — the reload uses syntactic order to match old to new
instances. Adding a third identical call between them treats the
new call as fresh; the existing two preserve state.

#### 13.17.11 Interaction with other reactive features

**With `when` clauses (§13.9):** an operator instance has no
`when` predicate of its own. To gate an operator's effect, gate
its output cell or its consumer. The author can also write a
gated wrapper operator that conditionally falls through:

```
operator conditional_smooth(source: Signal[f32], gate: Signal[bool], clock: Signal[u64]) -> Signal[f32]:
  derived effective: f32 = if gate then (source |> smooth(rate: 0.1, clock: clock)) else source
  effective
```

**With cycles (§13.11):** operator-internal cells participate in
the same cycle-detection rules. A recurrent inside an operator
acts as a delay element identical to a top-level recurrent.

**With the per-publish DAG (§13.11.3):** each operator instance's
internal cells contribute their evaluation nodes to the per-publish
DAG. Operators do not cross publish boundaries — all internal
evaluation happens within a single publish.

**With reactive transparency (§13.12.2):** operator bodies are
*not* reactive-transparent. Reading a cell-bound parameter reads
through the reactive engine (provenance tracked at the call site).
Calls to other operators inside the body create further
instantiations; calls to `fn`s inside the body remain
reactive-transparent in the standard way.

**With dynamic-size types (§13.12.4, §14.3.5):** operator-internal
recurrents may hold dynamic-size types (`Vec[T]`, etc.). Storage
follows the same pool-with-handle mechanism. The operator's
instance-specific allocation contributes to per-type pool sizing
in graph specification.

**With streams (§13.18) and effects (§13.19):** operators share
the same composition surface (`|>` pipe form, instance identity,
parameter rules, generics, visibility) as effects, and produce or
consume streams via the standard stream operators (§13.18.7). The
distinction is semantic role: operators perform pure reactive
transforms with no outside-world side effects, while effects align
program state with external reality through the reconciliation
model. The two compose naturally — an operator wrapping an effect
expresses domain patterns like debounced fetches (`url |> debounce
|> fetch`), cached requests, and retried operations.

#### 13.17.12 Diagnostics

Normative diagnostic classes for operator usage:

**`|>` applied to a non-operator:**

```
error: `|>` requires an operator on the right-hand side
  --> let bar = 0.0 |> some_fn
                     ^^^^^^^^ `some_fn` is a `fn`, not an operator
  hint: use function call syntax: `some_fn(0.0)`
```

**Operator missing first positional parameter:**

```
error: operator `smooth` has no positional parameter to bind from `|>`
  --> derived bar: f32 = source |> smooth(rate: 0.1)
                                   ^^^^^^ no positional parameter declared
  hint: either pass the upstream cell as the first positional argument,
        or declare a positional `Signal[T]` parameter on the operator
```

**`Signal[T]` parameter passed a non-cell, non-literal value:**

```
error: cannot pass value of type `f32` to `Signal[f32]` parameter
  --> smooth(source: some_value, rate: 0.1, clock: tick)
                     ^^^^^^^^^^ expected `Signal[f32]`, found `f32`
  hint: literals are wrapped as implicit constant signal cells
        (compile-time-fixed `Signal[T]` values) automatically; this expression
        cannot be wrapped — use a `signal`, `derived`, or `recurrent` declaration
```

**`signal` or `attr` declared inside an operator body:**

```
error: `signal` declarations are not permitted inside operator bodies
  --> operator foo(source: Signal[f32]) -> Signal[f32]:
        signal internal: f32 = 0.0
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  hint: operator-internal cells must be `recurrent` or `derived`. For
        per-instance configuration, use a parameter; for stateful memory,
        use `recurrent`.
```

**Final expression type mismatch with declared return type:**

```
error: operator body returns `i32` but declared return type is `Signal[f32]`
  --> operator bad(source: Signal[f32]) -> Signal[f32]:
        ...
        42
        ^^ this expression has type `i32`
  hint: the final expression's value type must match the operator's return value type
```

### 13.18 Streams

A *stream* is a reactive primitive for append-only event sequences with
a fixed-size ring buffer. Streams complement the value-cell primitives
(signal, attr, recurrent, derived) by expressing event-shaped flows that
those primitives cannot represent cleanly: discrete sequences of values
arriving over time, possibly faster than consumers can process them,
where consumers care about each event rather than the latest value.

Streams are first-class reactive cells. They participate in the publish
cycle (§13.10), in cell identity for hot reload (§13.15), and in the
graph specification (§15.4). They are not values that flow through
ordinary expressions — they are cells with read and write surfaces
distinct from signals.

#### 13.18.1 Concept

A stream carries an append-only sequence of typed events. Each event is
produced by a *producer* (a stream-emitting operator chain or a host-
side push) and observed by zero or more *consumers* (operators or
deriveds reading from the stream).

The stream's storage is a fixed-size ring buffer of typed slots,
allocated once at the stream's declaration site. The buffer's capacity
is part of the stream's type. When the buffer fills and a producer
pushes another event, the stream's *policy* (§13.18.3) determines
whether the new event displaces the oldest (`ring`) or is rejected
with failure (`gate`).

Streams have scope-bound lifetime: a stream lives as long as the
declaration's enclosing scope and is freed when that scope dies. There
is no garbage collection of stream events; the ring buffer is one
fixed memory region for the stream's entire lifetime, reused as events
arrive.

Streams are distinct from `Signal[T]`:

- `Signal[T]` has a single current value, always defined (§13.2.6).
- `Stream[T]` has zero or more pending events, each consumed
  independently. There is no "current value" of a stream; consumers
  project a stream to a signal explicitly (§13.18.7).

#### 13.18.2 Declaration

```
stream policy[capacity]? name: Type? = source
```

- **`policy`** is one of the policy keywords `ring` or `gate`
  (§13.18.3). Mandatory; the declaration has no default policy.
- **`[capacity]`** is an optional positive integer literal specifying
  the ring buffer's slot count. When omitted, defaults to `1024`.
- **`name`** is a snake_case identifier naming the stream.
- **`Type`** is the element type of the stream. Optional when
  inferable from the source expression's element type.
- **`source`** is a stream-producing expression — typically an
  operator chain ending in a stream-producing operator (§13.18.7),
  another stream, or a merge of streams.

Examples:

```
stream ring[2048] user_clicks: ClickEvent = button_press |> to_stream

stream gate[256] db_writes = pending_writes        // type inferred

stream ring url_changes: Url = current_url |> to_stream |> skip_first
```

The third example uses the default capacity (1024) and an inferred
element type.

**Where streams may be declared.** Streams are reactive declarations.
They may appear in the same scopes as other reactive declarations
(§13.2): at module level, inside node bodies, inside connection
bodies, inside operator bodies, inside effect bodies (§13.19). They
may not appear inside function bodies (§13.12.2 — functions are
reactive-transparent).

**Visibility.** Module-level streams carry the standard three-level
visibility (§10): `public`, `shared` (default), `private`. Streams
inside node, connection, operator, or effect bodies inherit the
enclosing declaration's visibility.

#### 13.18.3 Stream types

A stream's type encodes its element type, its policy, and its
capacity. The type hierarchy:

```
Stream[T]                 // abstract base; polymorphic over policy and capacity
RingStream[T, N]          // concrete: ring policy, capacity N
GateStream[T, N]          // concrete: gate policy, capacity N
```

`Stream[T]` is the abstract type for any stream of element type `T`,
regardless of policy or capacity. It is used in operator and function
signatures that accept any stream:

```
operator map[T, U](source: Stream[T], f: T -> U) -> Stream[U]:
  ...
```

`RingStream[T, N]` and `GateStream[T, N]` are concrete types — every
stream value belongs to one of these at runtime. They are used when
the policy or capacity matters at the type-system level, such as in
operators or effects that constrain their inputs:

```
operator persist[T](writes: GateStream[T]) -> ...:
  // requires gate policy — persist must never silently drop writes
  ...
```

**Policy as a constraint.** An operator parameter typed
`RingStream[T, N]` accepts only ring streams; passing a gate stream
is a type error. An operator parameter typed `Stream[T]` is
polymorphic over both policies. Operators that preserve policy
(e.g., `map`, `filter`) declare both input and output in terms of
the same abstract `Stream[T]`; the implementation propagates the
concrete policy from input to output.

**Capacity as a type parameter.** Capacity is part of the type for
analysis and pool sizing (§14.3.5) but does not generally appear in
operator constraints. An operator that accepts `Stream[T]` accepts
any capacity. An operator that needs a specific capacity bound
declares it explicitly: `Stream[T]` for any, `RingStream[T, N]` for
a specific capacity `N`, or with a trait bound expressing capacity
relationships if relevant.

**Conversion between concrete and abstract.** A `RingStream[T, N]`
value is implicitly usable wherever a `Stream[T]` is expected. The
abstract type is a supertrait of both concrete types; the conversion
is zero-cost at runtime (the stream value is unchanged; only the
type-system view is widened).

#### 13.18.4 Sink types

A *sink* is the write-side view of a stream. The stream and its
sink share the same underlying ring buffer; they differ only in
access mode:

- A **Stream** is the *read* view. Consumers observe events through
  cursors.
- A **Sink** is the *write* view. Producers push events into the
  buffer.

```
Sink[T]                   // abstract base
RingSink[T, N]            // concrete: ring policy, capacity N
GateSink[T, N]            // concrete: gate policy, capacity N
```

Sinks appear primarily in effect declarations (§13.19.4): an
effect's `desired:` block may declare a sink that the program writes
to, with the corresponding stream view held by the effect's host-
side reconciler.

Outside of effect declarations, sinks appear when a stream's
producer-side handle is exposed explicitly — for instance, an
operator that constructs a stream and returns both its stream view
(for consumers) and its sink view (for the operator's caller to
push into) as a record.

**Pushing into a sink.** Sinks are not written via assignment.
Producers push into a sink by piping a stream into it directly via
`|>` (§13.17.7 Case 3):

```
let events_to_log: Stream[LogEntry] = ...
let log_sink: Sink[LogEntry] = ...
events_to_log |> log_sink                // forwards stream into sink
```

The pipe establishes a forwarding subscription: each event observed
from the stream is pushed into the sink. The expression's value is
the unit type `()`; the subscription lives for the enclosing scope.

A single sink may receive from multiple stream-sources via multiple
pipe-into-sink expressions (multi-producer pattern). The receiving
sink's ring buffer is shared; events from all producers arrive in
their publish-commit order.

**No standalone sink declaration.** Sinks are not declared with a
top-level `sink` keyword. A sink exists only as the write-side
counterpart of a declared stream. When a `stream` declaration is
made, both views are implicit — the declared name binds to the
Stream view; the Sink view is accessed via the producer-side
machinery (the source expression in the declaration, or the sink
field of an effect's desired block).

#### 13.18.5 The `Cell` trait

`Cell[T]` is the common abstraction over the reactive cell types
that carry values of type `T`. It is implemented by `Signal[T]`,
`Stream[T]`, and `Sink[T]` (and their concrete sub-types).

```
Cell[T]                       // abstract base — any reactive cell carrying values of type T
  Signal[T]                   // single current value (§13.2.8)
  Stream[T]                   // append-only event sequence (§13.18.3)
    RingStream[T, N]
    GateStream[T, N]
  Sink[T]                     // write-only stream end (§13.18.4)
    RingSink[T, N]
    GateSink[T, N]
```

`Cell[T]` is used in operator and function signatures that accept any
reactive cell containing values of `T`, without constraining which
kind:

```
operator monitor[T, C: Cell[T]](source: C) -> Signal[bool]:
  // works with any Signal, Stream, or Sink carrying T
  ...
```

The trait has no required methods at the language level; it is a
marker indicating participation in the reactive system as a typed
cell. Concrete behavior (current-value reads, event observation,
push semantics) is specific to each implementing type and is
expressed through the type's own methods and operators.

**Direction-specific sub-traits.** Where an operator needs to
constrain by direction, two sub-traits refine `Cell[T]`:

- `Readable[T]: Cell[T]` — implemented by `Signal[T]` and `Stream[T]`
  (the read views).
- `Writable[T]: Cell[T]` — implemented by `Sink[T]` (and by
  `Signal[T]` from the host's perspective, though program code
  cannot write to signals — see §13.2.7).

These finer-grained traits are stdlib-provided; most operators use
the parent `Cell[T]` or a concrete type directly.

**Why a trait, not a union type.** Ductus does not have ad-hoc union
types at value position (§5.2 `dyn` is for trait objects). A common
abstraction over heterogeneous reactive cells must be expressed as
a trait. `Cell[T]` plays that role.

**Use in generic signatures.** Operators returning a `Cell[T]`
declare the concrete kind through their return type. For example,
`to_stream` returns `Stream[T]` (a concrete `Cell[T]`); `to_signal`
returns `Signal[T]`. The trait is rarely the return type of an
operator — concrete types carry more information and are preferred
unless the operator genuinely produces a polymorphic output.

#### 13.18.6 Observation cells

Every stream automatically exposes a set of derived signal cells
describing its state. These cells are accessed via field syntax on
the stream value:

```
stream ring[1024] events: LogEntry = source

derived current_pressure: f32 = events.pressure
derived dropped_so_far: i64 = events.dropped_total
derived backed_up: bool = events.is_full
```

The full observation surface, available on every stream:

| Cell | Type | Meaning |
|---|---|---|
| `pending_count` | `Signal[i32]` | Events in the buffer not yet observed by every cursor |
| `pressure` | `Signal[f32]` | `pending_count / capacity`, range `0.0..1.0` |
| `is_full` | `Signal[bool]` | `true` when `pending_count == capacity` |
| `dropped_total` | `Signal[i64]` | Cumulative count of events displaced by overflow (ring only; always `0` for gate) |
| `rejected_total` | `Signal[i64]` | Cumulative count of pushes rejected by overflow (gate only; always `0` for ring) |
| `last_overflow_at` | `Signal[Option[instant]]` | Timestamp of the most recent overflow event, or `none` if never |

These cells are ordinary `Signal[T]` cells for all purposes —
participating in the publish cycle, in derived dependencies, in hot
reload identity. They are not separately declared in user code; the
compiler synthesizes them as part of the stream's storage.

**Pressure-driven self-throttling.** The observation cells let
producer-side code react to consumer lag. A producer that wishes to
self-throttle reads the stream's `pressure` (or `is_full`) signal
and gates emission based on it — e.g., by feeding the signal into a
conditional `derived` upstream of the stream-producing chain, or by
using a stream operator that consults the back-pressure signal.

The exact throttling pattern depends on the producing chain's
shape; the stdlib provides operators that combine well with the
observation surface (e.g., `throttle` per §13.18.7 with a
pressure-derived gating signal).

**Gate-side back-pressure.** For `gate` streams, the `rejected_total`
signal lets the producer-side code observe rejections and take
corrective action (retry, log, surface error). The pattern is the
same shape, reading `rejected_total` or `is_full` instead of
`pressure`.

#### 13.18.7 Stream operators

Operators that produce, transform, or consume streams are stdlib
primitives. All use the standard operator-application syntax
(`|>` chains or function-call form per §13.17.6).

**Signal-to-stream bridge:**

```
operator to_stream[T](source: Signal[T]) -> Stream[T]:
  // emits initial value as first event;
  // emits each subsequent committed value of source as a new event
  ...
```

The semantics: at the moment of stream creation, the source signal's
current value is emitted as event 0; thereafter, each commit of a new
value by the source (per the publish cycle) appends an event. Same-
value commits do not emit (per the value-change semantics of §13.2.4.4).

The output stream's policy defaults to `ring` and capacity to `1024`;
overrides are supplied at the declaration site (`stream ring[N]
name = sig |> to_stream`).

**Stream-to-signal bridge:**

```
operator to_signal[T](source: Stream[T], default: T) -> Signal[T]:
  // returns a Signal[T] whose value is the latest observed event,
  // or `default` if no event has been observed yet
  ...
```

The default is required because signals must always have a defined
value (§13.2.6 initial value rules; §13.9.7 cell-value reads). The
returned signal updates on each new event.

**Skip family:**

```
operator skip[T](source: Stream[T], n: i32) -> Stream[T]:
  // drops the first `n` events observed from source

operator skip_first[T](source: Stream[T]) -> Stream[T]:
  // equivalent to skip(1)
```

The most common use of `skip_first` is to drop the initial-value
event emitted by `to_stream`, leaving only true changes:

```
stream ring changes: Url = current_url |> to_stream |> skip_first
```

**Projection operators** (Stream → Signal):

```
operator count[T](source: Stream[T]) -> Signal[i64]:
  // running count of events observed

operator fold[T, A](source: Stream[T], init: A, f: (A, T) -> A) -> Signal[A]:
  // running accumulator

operator any[T](source: Stream[T], pred: T -> bool) -> Signal[bool]:
  // true once any event satisfies pred

operator all[T](source: Stream[T], pred: T -> bool) -> Signal[bool]:
  // true if every event so far satisfies pred (true initially)
```

These produce signals from streams without requiring a default value
because the predicate or accumulator establishes the initial signal
value structurally (`0` for `count`, `init` for `fold`, `false` for
`any`, `true` for `all`).

**Transformation operators** (Stream → Stream):

```
operator map[T, U](source: Stream[T], f: T -> U) -> Stream[U]:
  // policy and capacity preserved from source

operator filter[T](source: Stream[T], pred: T -> bool) -> Stream[T]:
  // policy and capacity preserved from source

operator merge[
  T,
  const N: usize = A.capacity + B.capacity,
](
  a: RingStream[T, A],
  b: RingStream[T, B],
) -> RingStream[T, N]:
  // interleaves events from both sources in publish-commit order;
  // default capacity is the sum of input capacities, ensuring no
  // overflow if both inputs fill simultaneously

operator throttle[T](source: Stream[T], window: duration, clock: Signal[u64]) -> Stream[T]:
  // rate-limits events to at most one per window
```

Transformation operators that preserve policy and capacity do so by
construction: their output stream uses the same ring buffer
configuration as their input.

The `merge` operator uses a const-generic capacity parameter with a
default expression referencing the input streams' capacities (per
§2.3.6 const-generic default expressions). Callers may override the
capacity at the call site via turbofish if they have tighter bounds:

```
let merged: RingStream[Event, 1024] = merge::<Event, 1024>(a, b)
```

A separate `merge_gate` variant is provided for gate streams with
the same shape; cross-policy merges (one ring, one gate) are not
permitted (compile error — the result's overflow semantics would be
ambiguous).

**Stream-to-sink forwarding** is not an operator — it is expressed
directly via `|>` Case 3 (§13.17.7):

```
source_stream |> target_sink
```

The pipe establishes a forwarding subscription that lives for the
enclosing scope. There is no dedicated operator wrapper; the pipe's
type-directed dispatch handles the case when the RHS is a Sink.

#### 13.18.8 Policy as type

Stream policy is encoded in the type rather than as a runtime
attribute. This means operators that care about policy can constrain
their inputs at compile time, and the compiler catches incompatible
combinations.

**Policy-preserving operators.** Operators that transform a stream
without changing its overflow semantics (`map`, `filter`, `skip`)
preserve the policy type:

```
operator map[T, U](source: Stream[T], f: T -> U) -> Stream[U]:
  ...

// At a call site:
let mapped: RingStream[U, 1024] = (some_ring_stream: RingStream[T, 1024]) |> map(f)
let mapped2: GateStream[U, 256] = (some_gate_stream: GateStream[T, 256]) |> map(f)
```

The output's concrete policy matches the input's. The signature is
written in terms of the abstract `Stream[T]`; the compiler propagates
the concrete policy through.

**Policy-constraining operators.** Operators that require a specific
policy declare it concretely in the signature:

```
operator persist[T: Persistable](writes: GateStream[T]) -> EffectResult:
  // writes must be lossless; passing a RingStream is a type error
  ...

operator emit_telemetry[T: Telemetry](events: RingStream[T]) -> ():
  // ring is the right semantics for telemetry — losing oldest events on overload is acceptable
  ...
```

Passing the wrong policy stream is a compile error:

```
error: cannot pass `RingStream[Write, 1024]` to `GateStream[Write, _]` parameter
  --> writes |> persist
              ^^^^^^^^^
  hint: `persist` requires a gate stream because lost writes would be incorrect;
        the source stream uses ring policy. Either reconstruct the producing chain
        as a gate stream, or use a lossy-acceptable variant of `persist`.
```

This catches a class of errors that would otherwise surface only at
runtime as silent data loss.

#### 13.18.9 Consumer cursors

Each consumer of a stream maintains its own cursor — a position into
the ring buffer marking the oldest event the consumer has not yet
observed. Cursors are per-consumer state; two consumers reading the
same stream advance independently.

A consumer is any operator instance whose signature consumes the
stream (`Stream[T]` parameter). Each instantiation of such an
operator allocates a fresh cursor. Multiple consumers of the same
stream see the same events; each consumer observes the full sequence
from its point of attachment.

**Cursor identity.** A cursor's identity follows the consuming
operator instance's identity (§13.17.6.1). Two `|>` chains in
different scopes that both consume the same stream get distinct
cursors; one chain that is preserved across hot reload (via the
operator hot-reload rule §13.17.10) preserves its cursor.

**Buffer retention is policy-driven, not cursor-driven.** Cursors
are advisory: the ring buffer overwrites or rejects per its declared
policy regardless of cursor positions. A slow consumer (one whose
cursor lags far behind the head) does not hold back the buffer; the
buffer continues to fill, and under `ring` policy, slow consumers
will miss events as the buffer overwrites past their cursor.

When a cursor's position is overwritten by a `ring` policy advance,
the cursor automatically jumps forward to the oldest still-present
event. The consumer observes this as a gap — the `dropped_total`
signal increments by the number of skipped events. Consumers that
care about completeness must monitor `dropped_total` or use `gate`
streams.

**No cursor rewind.** Cursors only advance. There is no operation
to rewind a cursor to an earlier position; the buffer's events are
not persistently stored beyond the ring buffer's lifetime, and
events may have been overwritten.

#### 13.18.10 Memory model

A stream's storage consists of:

1. **The ring buffer.** A fixed-size array of `capacity` slots, each
   of `sizeof(T)` bytes. Allocated once at stream creation; freed
   when the stream's scope dies. Total size: `capacity * sizeof(T)`.
2. **The head pointer and per-stream metadata.** Counters for
   `pending_count`, `dropped_total`, `rejected_total`,
   `last_overflow_at`. Stored as ordinary reactive cells (§14.4).
3. **Per-consumer cursors.** One cursor per consumer instance.
   Stored as part of the consumer's per-instance state.

The ring buffer itself is allocated from a per-`(T, capacity)` pool
in the reactive state buffer (§14.3.5). All stream instances with
the same element type and capacity share a pool; each instance
occupies one buffer-sized slot within the pool. The compiler
enumerates stream declarations at compile time and computes the
per-`(T, capacity)` pool sizes statically.

Hot reload can extend or shrink these pools as stream declarations
are added or removed; the per-`(T, capacity)` pool mechanism is the
same as for other dynamic-size cell types (§14.3.5).

**No dynamic allocation.** Streams do not allocate memory at runtime
beyond their initial buffer. Producers write into pre-allocated
slots; consumers read from pre-allocated slots; cursors advance
through indices. The fixed buffer is the entire memory cost of the
stream.

#### 13.18.11 Hot reload

Stream hot reload preserves the ring buffer iff the stream's
*type signature* is byte-identical between old and new code. The
type signature comprises:

- The element type `T` (structurally identical, not just same-named).
- The policy (`ring` or `gate`).
- The capacity (the integer literal `N`).

When all three match, the ring buffer's contents survive the reload.
The cursors of consumers preserved by their own identity rules
(§13.17.10 for operator instances) continue from their previous
positions. The source expression (the `= source` part of the
declaration) may change freely; only the type signature gates
preservation.

When any of the three differs, the old buffer is discarded and a new
empty buffer is allocated. All cursors reset to the empty position
of the new buffer.

**`@reset_on_reload` annotation.** A stream declaration may carry the
`@reset_on_reload` annotation to opt out of buffer preservation
regardless of type-signature match:

```
@reset_on_reload
stream ring[1024] events: LogEntry = source
```

After any reload affecting this stream's declaration site, the
buffer is reset to empty. This is appropriate when buffered events
from the prior program version would be misinterpreted by the new
version's consumers.

**Annotation changes between reloads apply prospectively.** Adding
`@reset_on_reload` to a stream declaration that previously did not
carry it does *not* retroactively reset the current buffer; the
current reload preserves the buffer per the type-signature rule.
The new annotation takes effect on the *next* reload affecting this
stream's declaration site. Removing the annotation behaves
symmetrically: the current reload still resets the buffer (the old
annotation applied to the in-progress reload); subsequent reloads
preserve. This matches the precedent for `when` predicate changes
(§13.15.3): structural-metadata changes apply prospectively.

**Cursor identity across reload.** A consumer's cursor is preserved
when the consuming operator (or derived) instance is preserved per
its own identity rule. When the consumer is added (a new
instantiation appears in the new source), its cursor starts at the
current head — it observes only events arriving after the reload.
When the consumer is removed, its cursor is dropped.

**Reload-unsafe stream changes** require per-instance restart or
full-kernel restart per §13.15.4:

- Element type changes (incompatible structural change): per-
  instance restart — the affected stream and its consumers are
  recreated.
- Policy changes (`ring` ↔ `gate`): per-instance restart.
- Capacity changes: per-instance restart.

Implementations detect these during the diff phase.

#### 13.18.12 Restrictions

- **Streams may not appear inside function bodies.** Functions are
  reactive-transparent (§13.12.2); they have no place to host
  reactive declarations. A function that needs to produce events
  for downstream reactive consumption returns a value the caller
  feeds into an operator that emits a stream.
- **A stream's `source` expression must produce a stream.** Passing
  a signal directly is a type error — explicit conversion via
  `to_stream` is required (§13.18.7).
- **Cursors are not first-class values.** Programs cannot construct,
  store, or pass cursors. Cursors are implementation state of
  consuming operators; they are observable only through the
  consumer's eventual signal outputs.
- **No mid-publish stream observation.** Within a single publish
  cycle, a consumer observes the set of events committed by the
  end of producer evaluation; events emitted *during* the consumer's
  own evaluation are deferred to the next publish. This preserves
  the synchronous-dataflow semantics (§13.2.4.1).
- **Streams may not be passed to `kernel.write_signal` or
  `kernel.write_attr`.** Streams are not signal-shaped or attr-
  shaped cells. Host-side writes into a stream go through the
  dedicated host API (§13.14.8 `kernel.push_stream`).

#### 13.18.13 Diagnostics

Normative diagnostic classes for stream usage.

**Missing policy keyword in stream declaration:**

```
error: stream declaration requires a policy keyword (`ring` or `gate`)
  --> stream my_events: Event = source
             ^^^^^^^^^^ no policy specified
  hint: streams must declare an overflow policy. Use `ring` for
        lossy-acceptable streams (overwrites oldest on full) or
        `gate` for lossless streams (rejects new pushes on full):
        `stream ring[1024] my_events: Event = source`
```

**Signal passed where Stream expected (missing `to_stream`):**

```
error: cannot pass `Signal[T]` to `Stream[T, _, _]` parameter
  --> stream ring[1024] events: Event = current_signal
                                        ^^^^^^^^^^^^^^ expected a stream
  hint: signal-to-stream conversion is explicit. Apply `to_stream`:
        `stream ring[1024] events: Event = current_signal |> to_stream`
```

**Stream read as a value (missing `to_signal`):**

```
error: cannot read `Stream[T, _, _]` as a value
  --> derived latest: Event = events
                              ^^^^^^ this is a stream, not a value cell
  hint: streams have no current value. Project to a signal via
        `to_signal`, or fold the stream:
        `derived latest = events |> to_signal(default_event)`
```

**Policy mismatch in pipe chain:**

```
error: cannot pass `RingStream[Write, 1024]` to `GateStream[Write, _]` parameter
  --> writes |> persist
                ^^^^^^^
  hint: `persist` requires a gate stream because lost writes would
        be incorrect; the source stream uses ring policy. Either
        reconstruct the producing chain as a gate stream, or use a
        lossy-acceptable variant of `persist`.
```

**Assignment to a sink:**

```
error: cannot assign to sink `outbound`
  --> ws.outbound = some_message
      ^^^^^^^^^^^^^^^^^^^^^^^^^^
  hint: sinks receive events through stream forwarding, not assignment.
        Pipe a stream into the sink: `stream_of_messages |> ws.outbound`.
```

**Stream declaration inside a function body:**

```
error: `stream` declarations are not permitted inside function bodies
  --> fn helper():
        stream ring[1024] events: Event = source
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  hint: streams are reactive declarations. Functions are
        reactive-transparent (§13.12.2) and cannot host reactive
        declarations. Move the stream to a module, node, operator,
        or effect scope.
```

### 13.19 Effects

An *effect* is a reusable, cell-allocating reactive construct that
describes a desired alignment between program state and external
reality, declared with the `effect` keyword. Effects are the
mechanism by which Ductus programs interact with the outside world:
network requests, persistent storage, long-lived resources (sockets,
audio sessions, file handles), event subscriptions, and any other
domain where program state must be reflected in or sourced from a
runtime environment.

Effects are distinct from `node`, `operator`, and `fn`:

- `fn` is reactive-transparent pure computation (§11, §13.12.2).
- `operator` is a stateful reactive transform from cells to cells
  (§13.17), pure with respect to outside reality.
- `node` is a topological participant in the reactive graph
  (§13.3), composed via parts and connections.
- `effect` describes outside-world alignment — the host-interpreted
  bridge between program state and runtime environment. Composes
  with operators via `|>`; not placed via node-style placement.

Effects are first-class typed values. An effect declaration named
`fetch` introduces both a type `fetch` and a constructor `fetch`;
instances are values of that type with addressable cells.

#### 13.19.1 Concept

Earlier reactive systems and effect libraries (React `useEffect`,
Elm `Cmd`, Haskell `IO`, Solid `createResource`, Angular
`rxResource`) express effects as *invocations*: a function body
runs in response to a trigger, performs an action, and produces a
result. The invocation model handles request/response shapes
cleanly but struggles with long-lived resources, bidirectional
streams, and effects whose lifecycle is entangled with program
state.

Ductus effects use a *reconciliation* model instead. An effect
declaration consists of two record-shaped blocks:

- **`desired:`** — cells the program writes (or that flow from the
  effect's parameters); the host reads them.
- **`observed:`** — cells the host writes; the program reads them.

The host registers a *reconciler* keyed by the effect's type name
(§13.19.14). On each publish, the reconciler reads the effect
instance's parameters and desired cells, performs whatever real-
world operations align reality with the desired state, and writes
the actual outcome into the observed cells. The program reads the
observed cells through the standard reactive machinery.

This model unifies request/response (a single-value `desired.request`
that the host satisfies once) with long-lived resources (continuously
maintained alignment, e.g., a websocket whose `desired.should_be_open`
toggles open and closed) under a single primitive. The mental shift
from "fire an action" to "maintain alignment" is the model's central
discipline; it pays for itself by absorbing cancellation, restart,
lifecycle, and resource cleanup into a single mechanism (§13.19.11
hot reload; §13.19.12 lifetime).

The historical rationale for not having a dedicated effects construct
appears in the revised §13.1; the design space that justifies the
present construct is laid out there.

#### 13.19.2 Declaration

```
effect name[GenericParams]?(params...):
  desired:
    cell_declarations...
  observed:
    cell_declarations...
```

- **`name`** is a snake_case identifier serving as both the effect's
  type name and its constructor name (§13.19.8).
- **`GenericParams`** are optional type parameters with optional
  trait bounds (§3, §5), parallel to operators (§13.17.8).
- **`params`** is a comma-separated parameter list (§13.19.3).
- **`desired:`** is an optional block declaring cells the host's
  reconciler reads (§13.19.4).
- **`observed:`** is an optional block declaring cells the host's
  reconciler writes (§13.19.5).

Cell declarations inside each block carry an explicit role keyword
matching the cell's kind, parallel to the language's other reactive
declarations:

- `derived` (in `desired:`) — parameter-tracking reactive expression
  (§13.19.4).
- `sink` (in `desired:`) — write-only stream end (§13.19.4).
- `signal` (in `observed:`) — host-written value cell (§13.19.5).
- `stream` (in `observed:`) — host-written event sequence (§13.19.5).

No other declaration kinds are permitted inside the blocks.
Reactive declarations not listed above (e.g., `recurrent`, `attr`,
top-level `signal`) cannot appear inside an effect's body; stateful
behavior wrapping an effect is expressed via wrapping operators
(§13.19.15).

At least one of `desired:` or `observed:` must be present (an effect
with neither would have no surface at all). The blocks may be
declared in either order; the canonical order is `desired:` first,
`observed:` second.

Effects carry visibility modifiers (§13.19.10): `public`, `shared`,
`private`.

Effects do not return a value in the operator sense. They evaluate
to themselves — the instance value, accessed through the binding name
or through expression position (§13.19.13).

```
effect fetch(url: Signal[Url], method: Method = Method::GET):
  desired:
    derived request: Request = Request { method: method, target: url }
  observed:
    signal response: Option[Response] = none
    signal error: Option[FetchError] = none
    signal in_flight: bool = false
```

A minimal request/response effect. The `desired:` block declares a
parameter-tracking `derived` cell expressing the request; the
`observed:` block declares three host-written signal cells.

```
effect websocket(url: Signal[Url]):
  desired:
    sink ring[1024] outbound: Message
  observed:
    stream ring[1024] inbound: Message
    signal is_open: bool = false
    signal last_error: Option[WSError] = none
```

A long-lived resource effect with bidirectional message flow. The
`desired:` block declares a sink the program pushes into; the
`observed:` block declares a stream of inbound messages plus two
signal cells.

#### 13.19.3 Parameters

Effect parameters are cell-bound or value-typed, with the same rules
as operator parameters (§13.17.3):

**Cell-bound parameters** (`name: Signal[T]`, `name: Stream[T, P, N]`,
`name: <other Cell[T] type>`):
- Bind to a reactive cell at instantiation.
- The host's reconciler reads the cell's current value (or observes
  events, for stream parameters) during each invocation in which
  the parameter is dirty.
- Inside the effect's `desired:` cell expressions, the parameter is
  treated as a cell of value type `T` (auto-deref per §13.17.3.1).

**Value parameters** (`name: T`):
- Snapshotted at instantiation. Fixed for the effect instance's
  lifetime.
- Used for configuration values that do not vary at runtime (HTTP
  method, content type, retry budget).

**Defaults** are allowed on value parameters (§3.5.4 ordering rules
apply); not on `Signal[T]` parameters in v1.

**Pipe target.** The first positional parameter is the implicit
pipe target (§13.17.7 generalized to effects). For an effect
intended to be used with `|>`, authors place the primary upstream
cell as the first parameter:

```
effect fetch(url: Signal[Url], method: Method = Method::GET):
  ...

// usage:
let response = current_url |> fetch(method: Method::POST)
```

**Stream parameters.** Parameters may be of stream type
(`Stream[T]`, `RingStream[T, N]`, `GateStream[T, N]`). The host's
reconciler observes events from the parameter stream:

```
effect log(entries: GateStream[LogEntry, 4096]):
  // no desired, no observed — pure fire-and-forget consumer

// usage:
log_events |> log
```

**Reactive composite parameters.** Parameters may be reactive
composites (§13.2.9), in which case each constituent field is
tracked independently for dirty propagation to the reconciler.

#### 13.19.4 Desired block

The `desired:` block declares cells that the host's reconciler reads
to determine what alignment to perform. Two cell forms are permitted,
each introduced by an explicit role keyword:

**`derived` cells** — parameter-tracking reactive expressions. The
cell's value is the expression's value, recomputed reactively when
any input cell changes:

```
desired:
  derived request: Request = Request { method: method, target: url }
  derived auth_header: Option[string] = current_token |> as_bearer
```

The form matches a regular `derived` declaration (§13.2.3): the
expression is pure and reactive, reads from parameters and any
in-scope cells, and the cell value updates when those inputs change.
The host's reconciler reads the cell's current value on each
invocation. Program code can also read `derived` cells in `desired:`
via the standard access path (`f.request` — see §13.19.7), but
cannot write to them.

**`sink` cells** — write-only stream ends. The program pushes events
into the sink by piping a stream into it via `|>` (§13.17.7
Case 3); the host's reconciler consumes events from the paired
Stream view:

```
desired:
  sink ring[1024] outbound: Message
  sink gate[256] outgoing_writes: PendingWrite
```

The declaration shape parallels the `stream` declaration (§13.18.2)
— policy keyword (`ring` or `gate`), optional capacity in brackets
(defaulting to 1024), name, element type. The difference is the
leading `sink` keyword instead of `stream`, and the absence of an
`= source` clause: a sink's events come from program-side pipe-
into-sink expressions, not from a declared source.

**Cell name uniqueness.** Within a single `desired:` block, cell
names must be distinct. Cells in `desired:` may not share names
with cells in the same effect's `observed:` block (§13.19.6).

**Host-side semantics.** The reconciler reads `derived` cells on
each invocation where any of them is dirty. For `sink` cells, the
reconciler consumes the buffered events in order. The reconciler is
responsible for maintaining the alignment between the desired state
and the external environment.

#### 13.19.5 Observed block

The `observed:` block declares cells that the host's reconciler
writes. The program reads these cells through the standard reactive
machinery. Two cell forms are permitted, each introduced by an
explicit role keyword:

**`signal` cells** — host-written value cells. The program reads the
cell's current value; the host updates it via the host API
(§13.14.2):

```
observed:
  signal response: Option[Response] = none
  signal error: Option[FetchError] = none
  signal in_flight: bool = false
  signal is_open: bool = false
```

The declaration shape matches a regular `signal` declaration
(§13.2.1): name, value type, and an initial value. The initial value
is what the program reads before the host's first write — typically
a sentinel like `none` for `Option[T]` or `false` for `bool`.

The host writes signal cells via `kernel.write_signal` against the
effect instance ID (§13.14.2 per-instance form; §13.14.9 reconciler
protocol). Writes are dirty-tracked in the standard publish-cycle
way.

**`stream` cells** — host-written event sequences. The program
observes events the host appends via stream operators (§13.18.7);
the host pushes events via the host API (§13.14.8):

```
observed:
  stream ring[1024] inbound: Message
  stream gate[256] notifications: Notification
```

The declaration shape parallels the top-level `stream` declaration
(§13.18.2), but with no `= source` clause — the source is the host's
reconciler pushing events via `kernel.push_stream`. Policy and
capacity work as in regular stream declarations.

The stream begins empty. Consumers in program code project the stream
to a signal via `to_signal`, or fold/count/filter/etc. via the
standard stream operators.

**Cell name uniqueness.** Within a single `observed:` block, cell
names must be distinct. Cells in `observed:` may not share names
with cells in the same effect's `desired:` block (§13.19.6).

**Program writes are forbidden.** Writing to an observed cell from
program code is a compile error:

```
error: cannot assign to cell `response` on effect instance
  --> f.response = some(custom_response)
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  hint: effect cells are not writable from program code. The host's
        reconciler is the sole writer of cells in `observed:`. To
        inject test data, construct a different effect instance or
        use a stub effect.
```

**Host-side semantics.** The host writes observed signal cells via
`kernel.write_signal` (§13.14.2) and pushes into observed stream
cells via `kernel.push_stream` (§13.14.8), keyed by effect instance
ID and cell name. Writes are dirty-tracked in the standard
publish-cycle way; downstream deriveds in program code re-evaluate
on the next publish.

#### 13.19.6 Reserved keywords

The identifiers `desired` and `observed` are reserved as block
introducers inside `effect` declarations. Cell declarations inside
either block cannot use these names:

```
error: cell name `desired` is reserved inside an effect's blocks
  --> effect example():
        desired:
          derived desired: bool = false
                  ^^^^^^^ this name is reserved
  hint: `desired` and `observed` are reserved within `effect`
        declarations as block introducers. Choose a different cell
        name.
```

Outside of effect declarations, `desired` and `observed` are
ordinary identifiers and may be used freely (as variable names,
function names, etc.). The reservation is scoped to the effect
declaration body.

**Cross-block name collision.** An effect cannot declare cells with
the same name in both `desired:` and `observed:`:

```
error: cell name `target` appears in both `desired:` and `observed:` of effect `example`
  --> effect example():
        desired:
          derived target: Url = some_expr
        observed:
          signal target: Url = "..."
                 ^^^^^^ duplicate name
  hint: cells across `desired:` and `observed:` share a flat
        namespace via the access path `instance.field`. Rename one
        of the cells to avoid the collision.
```

The collision rule supports unambiguous flat access: `f.field`
resolves to whichever block declares `field`, with the compiler
enforcing that exactly one block does.

#### 13.19.7 Access rules

Effect instances expose their cells through a flat access path.
Cells in `desired:` and `observed:` share a single namespace at the
instance:

```
let f = current_url |> fetch
let req = f.request           // reads desired.request (a `derived` cell)
let r = f.response            // reads observed.response (a `signal` cell)
let loading = f.in_flight     // reads observed.in_flight
```

The cell-name collision rule (§13.19.6) ensures `f.<name>` is
unambiguous.

**No write-to-cell from program code.** Effect cells are not writable
from program code via assignment. The only program-side writes are
pipe-into-sink expressions (§13.17.7 Case 3). Writing to any effect
cell — signal, stream, or derived — via assignment is a compile
error:

```
error: cannot assign to cell `response` on effect instance
  --> f.response = some(custom_response)
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  hint: effect cells are not writable from program code. To control
        the effect's behavior, change the upstream signal(s) bound
        to its parameters, or pipe a stream into the effect's
        sink(s) via `|>`.
```

**Pushing into a sink:**

```
let ws = current_url |> websocket
my_outgoing_messages |> ws.outbound
```

Piping a `Stream[T]` into a `Sink[T]` establishes a forwarding
subscription. The sink is accessed on the effect instance by name
(`ws.outbound`). Multiple pipes may target the same sink
(multi-producer pattern); their events arrive in publish-commit
order.

**Reading a stream:**

Streams are consumed via stream operators, not via direct value
reads:

```
let ws = current_url |> websocket
let latest = ws.inbound |> to_signal(empty_message)    // project to signal
let total = ws.inbound |> count                         // running count
let recent_text: stream ring[64] = ws.inbound |> filter(is_text)
```

The stream value is the program-readable cursor view; the program
attaches operators to consume events.

**Reading observation cells of a stream:**

Stream cells expose their observation surface (§13.18.6) via field
access on the stream value:

```
let ws = current_url |> websocket
let pressure = ws.inbound.pressure
let dropped = ws.inbound.dropped_total
```

These reads project the synthesized signals describing the stream's
state.

#### 13.19.8 Effect instance identity

An effect instance is identified by its enclosing scope, its effect
type name, and its argument bindings — the same scheme as operator
instances (§13.17.6.1).

Two `|>` chains in different scopes (different modules, different
node bodies, different placements, different Repeat elements) that
both instantiate the same effect type produce distinct instances
with independent desired/observed cells and independent host-side
reconciler state.

Effect instances do not have user-assignable names in the language
sense. Binding the instance to a `let` or `derived` names the
*instance value* (the composite); for hot-reload-identity purposes
(§13.19.11), the same identity scheme as operators applies, with
tolerance for positional moves within the same scope. The binding
name has no role in identity.

**Type and constructor unified.** The effect's declared name serves
both as the type name (used in operator and function signatures,
type annotations, generic bounds) and as the constructor (used in
pipe chains and function-call form to instantiate). This parallels
operators (§13.17) and contrasts with nodes (§13.3), which separate
type names (PascalCase) from placement syntax.

```
effect fetch(url: Signal[Url]):
  ...

// Used as type:
operator render_fetch_card(f: fetch) -> Signal[VNode]:
  ...

// Used as constructor:
let f = current_url |> fetch
let f2 = fetch(current_url)
```

#### 13.19.9 Generic effects

Effects may take type parameters with optional trait bounds:

```
effect cached_fetch[T: Cacheable](
  url: Signal[Url],
  cache: Signal[Cache[T]],
):
  observed:
    signal value: Option[T] = none
    signal cache_hit: bool = false
    signal error: Option[FetchError] = none
```

Standard generics machinery applies (§3 traits, §2.2 inference).
Type parameters are resolved at the call site from argument types;
explicit instantiation via turbofish where ambiguous.

The reconciler registration (§13.19.14) is per-effect-type and per-
type-parameter-instantiation: a generic effect produces a distinct
reconciler-registration key per concrete instantiation, allowing
the host to dispatch on the resolved types.

#### 13.19.10 Visibility

Effects carry the standard three-level visibility (§10): `public`,
`shared` (default), `private`. Module-private effects are not
reachable from other modules; public effects may be re-exported.

```
public effect fetch(url: Signal[Url]):
  ...

private effect internal_health_check():
  ...
```

Visibility applies to the effect's type and constructor uniformly
(they share a name).

#### 13.19.11 Hot reload of effects

An effect instance is a scoped reload boundary, like an operator
instance. The cell-identity rules of §13.15.2 apply: each cell in
the effect's `desired:` and `observed:` blocks is identified by its
declared name and type within the effect's body.

**Reload-safe changes:**

- Changes to the initial-value expression of an `observed:` `signal`
  cell (the initial value affects only the pre-first-write
  behavior; existing live cells retain their committed values).
- Adding a new cell in `observed:` or `desired:` — new cells are
  initialized fresh per their declared initial value (signals) or
  empty (streams, sinks).
- Changes to `desired:` `derived` cell expressions — the cell
  re-evaluates on the next publish with new logic.

**Reload-unsafe changes** (per-instance restart per §13.15.4):

- Effect parameter signature changes (parameters added, removed,
  retyped, reordered).
- Cell type changes in `desired:` or `observed:` (a `signal` cell's
  value type changing from `i32` to `i64`, a stream's element type
  changing, etc.).
- Policy or capacity changes on `stream` or `sink` cells in the
  effect's blocks.
- Changes to a cell's role keyword (`derived` becoming `sink`,
  `signal` becoming `stream`, etc.).

When per-instance restart fires, the host's reconciler receives a
teardown call for the affected instances (releasing any host-side
resources tied to those instances), and new instances are
constructed under the new declaration. Other effect instances and
the rest of the kernel continue without restart.

**Call-site changes.** If a call site changes which effect is
invoked (`source |> fetch` becomes `source |> cached_fetch`), the
old instance's reconciler is torn down and the new instance is
constructed fresh. The two effects are treated as distinct
instances even if their cell shapes overlap.

**Call-site moves.** Pure positional moves within the same scope
preserve instance identity per the same rule as operators
(§13.17.10).

**Stream cells inside effects** follow the stream hot-reload rules
(§13.18.11): the buffer is preserved iff `(element type, policy,
capacity)` is byte-identical; `@reset_on_reload` on a stream cell
forces clear.

#### 13.19.12 Lifetime

An effect instance lives as long as its enclosing scope. When the
scope dies, the effect instance and all its cells are dropped per
§14.8. The host's reconciler receives a teardown call with the
instance ID, allowing it to release any external resources (open
sockets, audio sessions, file handles, pending requests).

Effect instance lifetimes follow the scope hierarchy:

- Module-level: lives for the program's lifetime.
- Inside a node: lives as long as the node instance is mounted.
- Inside a Repeat element: lives until the element key is removed
  from the iterated source (§13.5.4).
- Inside an operator body: lives as long as the enclosing operator
  instance.
- Inside another effect's `desired:` initial-value expression: lives
  as long as the outer effect instance (effects-inside-effects is
  restricted per §13.19.15; cell-derivation expressions referencing
  effects are subject to the same restriction).

**"Desired says no resource needed" is not the same as "effect
dies."** When a `desired:` cell's value implies the host should not
be holding a resource (e.g., a `derived target: Option[Url] = none`
that evaluates to `none` because the program hasn't supplied a URL),
the host tears down the resource but the effect instance is still
alive. The host remains ready to re-establish the resource if the
desired changes back. Only scope death causes instance teardown.

#### 13.19.13 Effects in `|>` chains

Effect instances participate in pipe chains identically to operators
(per the extended `|>` rule in §13.17.7):

```
let f = current_url |> fetch
```

The LHS of `|>` binds to the effect's first positional parameter.
The pipe's *value* is the effect instance — the composite of cells
in `desired:` and `observed:`, accessed via the rules in §13.19.7.

**No implicit projection.** An effect instance does not auto-project
to a single cell when used in pipe-out position. Operators downstream
of an effect either take the whole instance (declared with the
effect's type as the parameter type) or take a specific cell
projected by field access:

```
operator render_fetch_card(f: fetch) -> Signal[VNode]:
  // receives the whole composite
  ...

// At call site:
let card = (current_url |> fetch) |> render_fetch_card
let display = (current_url |> fetch).response |> render_response
```

The first form passes the composite; the second projects a specific
cell via field access before piping. The pipe operator carries
whatever the LHS evaluates to; no implicit projection occurs.

Naming the instance for downstream use is the common pattern:

```
let f = current_url |> fetch
let display = f.response |> render_response
let loading = f.in_flight |> as_loading_class
let err_msg = f.error |> as_error_message
```

This makes all cells accessible from a single binding via the flat
namespace rule (§13.19.7).

**Stream-typed observed cells** are accessed via the stream
operators (§13.18.7):

```
let ws = current_url |> websocket
let latest = ws.inbound |> to_signal(empty_message)
let messages_per_second = ws.inbound |> count |> rate_per_second
```

#### 13.19.14 Host integration

Effects are interpreted by the host. The host registers a
*reconciler* for each effect type via the host API (§13.14.7), keyed
by the effect's type name. The reconciler is a host-language object
(Rust struct, function table, or analogous construct) whose interface
mirrors the effect's declaration:

- **Read access** to the effect's parameter values and `desired:`
  cell values for a given instance.
- **Write access** to the effect's `observed:` cells via the host
  API (§13.14.2 `kernel.write_signal` for Signal cells, §13.14.8
  `kernel.push_stream` for Stream cells).
- **Lifecycle hooks**: instance creation (when the effect appears
  in the live graph), update (when parameters or desired cells
  change), and teardown (when the instance leaves scope).

The kernel invokes the reconciler at well-defined points in the
publish cycle:

1. After publish-and-swap, the kernel enumerates effect instances
   whose parameters or desired cells became dirty during the publish.
2. For each such instance, the kernel invokes the registered
   reconciler with the instance ID. The reconciler reads the new
   desired state and reconciles.
3. Reconciler writes into observed cells via the host API are
   dirty-tracked in the standard way; they take effect on the next
   publish.

**Reconciler idempotence.** Reconciler implementations are expected
to be idempotent in the reconciliation sense: re-applying the same
desired state produces the same alignment (no double-charging
side effects, no leaked resources, no duplicated requests). This
property is what allows the kernel to invoke the reconciler freely
without worrying about whether a previous invocation completed.

**Unregistered effect types.** If an effect type appears in the
graph specification with no registered reconciler, the kernel emits
a diagnostic at startup and refuses to enter the live state. Effects
must be registered before the kernel becomes live.

**Reconciler error reporting.** Reconciler errors (network failure,
resource exhaustion, etc.) are reported to the program through the
effect's `observed:` cells (typically a `signal error: Option[E] =
none` cell). The reconciler does not panic the kernel;
reconciler-internal errors are domain errors expressed through the
value track (§8).

#### 13.19.15 Restrictions

- **Effects-from-effects composition is deferred.** An effect's
  `desired:` block may not contain another effect's instantiation
  via `|>` chain in v1. The semantic complexity — cascading
  reconciler invocation order, partial-result lifecycles,
  cancellation cascades — requires its own design pass.
- **Effects do not use node-style placement syntax.** Effects are
  not topological participants; they are not placed via
  `EffectName name /` syntax. They are instantiated by appearance
  in expression position (pipe chains or function-call form).
- **Effects may not appear inside function bodies.** Functions are
  reactive-transparent (§13.12.2); they cannot host reactive
  declarations or instantiations. An effect-using function would
  need to be promoted to an operator.
- **No reactive declarations outside `desired:` and `observed:`
  blocks inside an effect body.** The effect's body consists only of
  the `desired:` and `observed:` blocks containing role-keyword cell
  declarations (`derived`, `sink`, `signal`, `stream`). Stateful
  behavior wrapping an effect — recurrent state, accumulators,
  retry logic — is expressed via wrapping operators, not via
  internal declarations in the effect.

#### 13.19.16 Diagnostics

Normative diagnostic classes for effect usage.

**Cell name is reserved inside an effect's blocks:**

```
error: cell name `desired` is reserved inside an effect's blocks
  --> effect example():
        desired:
          derived desired: bool = false
                  ^^^^^^^ this name is reserved
  hint: `desired` and `observed` are reserved within `effect`
        declarations as block introducers. Choose a different cell
        name.
```

**Cross-block cell name collision:**

```
error: cell name `target` appears in both `desired:` and `observed:` of effect `example`
  --> effect example():
        desired:
          derived target: Url = some_expr
        observed:
          signal target: Url = "..."
                 ^^^^^^ duplicate name
  hint: cells across `desired:` and `observed:` share a flat
        namespace via the access path `instance.field`. Rename one
        of the cells to avoid the collision.
```

**Write to effect cell from program code:**

```
error: cannot assign to cell `response` on effect instance
  --> f.response = some(custom_response)
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  hint: effect cells are not writable from program code. To control
        the effect's behavior, change the upstream signal(s) bound
        to its parameters, or pipe a stream into the effect's
        sink(s) via `|>` (§13.17.7 Case 3).
```

**Effect type with no registered reconciler:**

```
error: effect type `fetch` has no registered reconciler
  --> at kernel startup
  hint: every effect type appearing in the graph specification must
        have a reconciler registered via
        `kernel.register_reconciler("fetch", ...)` (§13.14.7) before
        the kernel enters the live state. Generic effects require
        one registration per concrete instantiation.
```

**Effect instantiation inside a function body:**

```
error: effect instantiations are not permitted inside function bodies
  --> fn helper(x: i32):
        let f = some_url |> fetch
                            ^^^^^ effect instantiation
  hint: functions are reactive-transparent (§13.12.2) and cannot host
        reactive declarations or instantiations. Promote `helper` to
        an operator, or perform the effect instantiation in a
        reactive scope (module level, node body, operator body, or
        effect cell expression).
```

**Effect instantiation inside another effect's body:**

```
error: effect instantiation inside another effect's body is not permitted
  --> effect outer(input: Signal[T]):
        desired:
          derived chained = input |> inner_effect
                                     ^^^^^^^^^^^^ effect-in-effect not allowed
  hint: effects-from-effects composition is deferred to a future
        revision (§13.19.15). Compose effects at the consumer site by
        feeding one effect's observed cells into another's parameters:
        `derived a = input |> first_effect`
        `derived b = a.result |> second_effect`
```

**Disallowed declaration inside an effect's blocks:**

```
error: only role-keyword declarations are permitted inside effect blocks
  --> effect example():
        desired:
          recurrent count: i32 = 0
          ^^^^^^^^^^^^^^^^^^^^^^^^^
            | on input: self.count + 1
  hint: effect blocks accept only `derived`, `sink` (in `desired:`)
        and `signal`, `stream` (in `observed:`). For stateful behavior
        wrapping an effect, use a wrapping operator (§13.17) that
        holds the recurrent state and consumes/produces effect cells.
```

**Effect appearing in node-placement position:**

```
error: effects cannot be placed via node-style placement syntax
  --> Fetcher f1                       // ✗ effect type used as placement
      ^^^^^^^^^^
  hint: effects are instantiated by appearance in expression position
        (pipe chains or function-call form), not via topological
        placement. Use `let f = some_url |> fetch` instead.
```

---

*End of §13.*

---

## 14. Implementation Model

This section specifies the contract between a Ductus program and its
runtime environment: how Ductus source is compiled, how the resulting
artifacts interact with the host kernel, and what guarantees the
implementation provides.

The contents of this section are *normative for implementations* of
Ductus, not for source-level code. Ductus programs do not depend on
these details directly; their behavior is determined by §§1–13. But
implementations must conform to the contracts specified here to ensure
that programs run correctly across implementations.

### 14.1 Compilation Modes

A conforming Ductus implementation provides two compilation modes:

**Interpreter mode** — Ductus source compiles to a compact bytecode
representation, executed by an interpreter embedded in the kernel.
Used for development workflows: fast iteration, hot reload, live
coding.

**Native mode** — Ductus source compiles, via a Rust intermediate
form, to a native executable. Used for production: maximum performance,
distributable artifact.

Both modes share the same frontend: lexer, parser, type checker,
semantic analysis. The frontend produces a typed intermediate
representation. The two modes diverge after this point: the bytecode
emitter targets the interpreter; the Rust emitter targets a Rust source
file that is then compiled by `rustc`.

The two modes produce equivalent observable behavior. A program that
runs correctly in interpreter mode produces the same output (modulo
performance and timing) in native mode. Implementations that diverge
observably between modes are non-conforming.

#### 14.1.1 The shared frontend

The frontend performs:

1. **Lexing and parsing** per `GRAMMAR.md`. Produces an AST.
2. **Name resolution and type checking** per §§2–10. Produces a typed
   AST with all generic instantiations resolved and all trait dispatch
   sites bound to concrete implementations.
3. **Borrow and ownership checking** per §11. Catches use-after-move,
   borrow conflicts, and other ownership violations.
4. **Reactive analysis** per §13. Identifies reactive declarations,
   computes dependency graphs, and extracts graph specification.
5. **Monomorphization** per §2.3. Resolves all generic instantiations
   in Ductus before lowering. Ductus's compiler does not delegate
   monomorphization to Rust; emitted code is fully concrete.

After these passes, the typed IR is consumed by one of the two
backends.

#### 14.1.2 Interpreter mode

The bytecode emitter lowers the typed IR to a stack-based bytecode.
The kernel includes a bytecode interpreter that executes this directly.
No native compilation step occurs.

Characteristics:

- Sub-second compilation time, suitable for live editing.
- Performance lower than native (a typical interpretation overhead is
  5–20× slower in tight loops; acceptable for development).
- Supports hot reload (§14.9): individual behaviors can be replaced
  in a running kernel without restarting.
- The bytecode format is implementation-internal and not stable across
  Ductus versions. It is not a distribution format.

#### 14.1.3 Native mode

The Rust emitter lowers the typed IR to Rust source code, which is then
compiled by the bundled `rustc` toolchain into a native executable. The
resulting binary is the distribution artifact.

Characteristics:

- Native performance, equivalent to hand-written Rust for the
  equivalent program.
- Compilation time is dominated by `rustc` (typically seconds to tens
  of seconds for non-trivial programs).
- Produces a single executable embedding both the compiled behaviors
  and the graph specification (§15.4).
- Does not support hot reload at runtime; rebuild is required to
  change the program.

The emitted Rust source is **fully monomorphic and Ductus-trait-free**
(with the narrow exception of Rust operator-overloading impls per
§15.5.2). Per
§15.5, the Rust emitter produces concrete struct definitions and
specialized function definitions per Ductus instantiation. Ductus's
trait system is not exported into the emitted Rust; trait dispatch
sites are resolved to direct function calls during frontend processing.

### 14.2 The Ductus CLI

A conforming implementation provides a command-line interface that
wraps the compilation modes. The CLI's interface is normative; specific
flag spellings may vary across implementations, but the operations are
required.

#### 14.2.1 Operations

- **`ductus run <file>`** — invokes interpreter mode. Compiles to
  bytecode and executes immediately. The kernel runs to program
  completion or until interrupted.

- **`ductus watch <file>`** — interpreter mode with file watching
  and hot reload. The kernel runs continuously; saved changes to the
  source trigger recompilation and reload of affected behaviors per
  §14.9.

- **`ductus build <file> [--release]`** — invokes native mode.
  Compiles via Rust to a native executable. `--release` enables
  optimization. The output is a single executable file.

- **`ductus check <file>`** — runs the frontend (lexing, parsing,
  type checking, ownership checking, reactive analysis) without
  invoking either backend. Produces diagnostics. Used by editor
  integrations (LSP).

- **`ductus fmt <file>`** — invokes the canonical formatter.
  Rewrites the source in normalized form.

- **`ductus test <file>`** — runs tests via interpreter mode.
  Optimized for fast feedback during development.

#### 14.2.2 Toolchain bundling

The CLI ships as a single binary that bundles or downloads on first
use:

- The Ductus frontend.
- The bytecode interpreter (part of the kernel).
- A `rustc` toolchain for native-mode builds.
- The Ductus stdlib and reactive kernel.

Users do not install `rustc` or `cargo` separately. The CLI does not
expose `cargo` directly; all Rust-toolchain invocations are internal.
Build output from `rustc` is suppressed in normal operation and
surfaced only when a compilation failure prevents Ductus's output
from being produced.

#### 14.2.3 Project layout

A Ductus project is a directory tree containing source files
(`.duc`). The CLI does not require a manifest file for single-file
programs (`ductus run file.duc` works on a lone file). Multi-file
projects use a manifest file specifying the entry point and any
external dependencies; the format of the manifest is
implementation-specific.

### 14.3 The Reactive State Buffer

The kernel maintains a contiguous memory region holding all reactive
cells of the running program. This region is the **reactive state
buffer**.

#### 14.3.1 Cell representation

Cells are 64-bit slots, each one `AtomicI64` in implementations
targeting threaded platforms (native, modern browsers with COOP/COEP
headers, etc.). The complete buffer has type `Arc<[AtomicI64]>` in the
reference Rust implementation.

A single cell directly stores any 8-byte-or-smaller primitive value
via bit reinterpretation:

| Type                   | Storage in cell                                                    |
|------------------------|--------------------------------------------------------------------|
| `bool`, `char`         | Single cell; value occupies the low bits, upper bits are zero.     |
| `i8`–`i64`, `u8`–`u64` | Single cell; value is sign- or zero-extended to 64 bits as needed. |
| `f32`, `f64`           | Single cell; value is bit-reinterpreted (transmute) as i64.        |
| `string`               | Single cell; value is a u64 handle into the string pool (§14.5).   |

Lossless conversion: reading and writing a cell preserves the
bit-exact value of any of these primitive types. `f64::from_bits` and
`f64::to_bits` perform the reinterpretation in the reference Rust
implementation.

#### 14.3.2 Multi-cell types

Types wider than 8 bytes (`i128`, `u128`, multi-field records used as
reactive values) occupy multiple consecutive cells in the buffer.

For example, an `i128` value occupies two cells: the low 64 bits in
cell N, the high 64 bits in cell N+1.

A record `Vec3 { x: f32, y: f32, z: f32 }` used as a reactive cell
value occupies one cell per field — three consecutive cells per
`Vec3`, each padded from 4 bytes to 8 bytes. This per-field layout
is the **canonical** layout; implementations conforming to the spec
must support it.

An implementation may optionally pack multiple sub-8-byte fields
into a single cell as an optimization (e.g., three f32s into one
8-byte slot with the fourth slot unused, or four `bool` fields into
the low bits of one cell). Such packing is an implementation
optimization and must not be observable from Ductus source — every
cell read and write through the kernel API must produce results
identical to the canonical per-field layout.

Records whose fields total more than 8 bytes each (e.g., fields of
type `i128` or nested non-Copy records) follow the same per-field
layout, with multi-cell types occupying their own consecutive cells
within the enclosing record's allocation.

#### 14.3.3 Triple-buffering

The reactive state buffer is **triple-buffered** to provide:

- Snapshot consistency across multiple cells for multi-cell values.
- Batched publication: writes accumulated in the back buffer commit
  atomically when the producer publishes.
- Wait-free reads from the consumer.

The arrangement is **single-producer, single-consumer (SPSC)**: one
*producer role* writes, one *consumer role* reads, mediated by three
buffer copies and an atomic current-pointer swap. The mapping of
these roles to physical threads, and the trigger that initiates a
publish, are implementation-defined; §14 specifies only the
mechanism. Typical native deployments map the producer role to the
host's main thread and the consumer role to one or more application
threads.

The kernel maintains three copies of the buffer:

- **Current**: the most recently published snapshot. Read by the
  consumer. Not written while serving as current.
- **Back**: actively being written by the producer. Not read by the
  consumer.
- **Pending**: a third buffer used to allow the producer to begin
  writing the next batch of state without waiting for the consumer.
  Rotation among the three is producer-managed.

##### 14.3.3.1 Publish operation

To publish accumulated writes, the producer performs:

1. Finalizes writes to the back buffer.
2. Atomically swaps the "current" pointer to point at the
   newly-written back buffer. The previous "current" rotates to
   become the next available back/pending buffer.

The publish operation runs on the producer role's thread. Its cost
is O(N) where N is the buffer size — the producer copies the
publishable state into the back buffer before the swap. The atomic
swap itself is O(1).

The "copy" here refers to the producer's carry-forward of unchanged
cells from the previous-current buffer to the back buffer, not a
re-copy of dirty cells. Dirty cells were written into the back buffer
incrementally between publishes per §13.10.1; the publish operation
copies forward only the cells the producer did not touch (so the new
back buffer is a complete snapshot). This carry-forward is O(N) in the
buffer size; the atomic swap itself is O(1).

The producer's per-publish cost is therefore O(N) memcpy + one
atomic operation. This cost is paid on the producer side, not on
the consumer side; consumers are unaffected.

When the producer chooses to publish (the trigger is specified in
§13.10 — the kernel's evaluation cycle) is outside the scope of this
section.

##### 14.3.3.2 Swap operation

The consumer, when it wants to read the latest published state,
performs **swap**:

1. Atomically load the current pointer.
2. Read cells from the buffer it points to.

The swap operation runs on the consumer role's thread. Its cost is
O(1) — one atomic load. The consumer never copies data; it reads in
place from the buffer the current pointer points to.

##### 14.3.3.3 Why three buffers

A two-buffer ping-pong would force the producer to wait for the
consumer to finish reading before publishing the next state. With
three buffers, the producer always has a buffer available to write
to that the consumer is not currently reading, even when the
consumer holds its reference into a snapshot for an extended period.
This preserves wait-free reads on the consumer side without
producer-side blocking.

##### 14.3.3.4 Multiple cross-thread observers

If a deployment requires multiple cross-thread observers (multiple
consumers reading the same producer's published state), the SPSC
triple buffer can be replicated — each observer maintains its own
SPSC channel against the producer. SPMC variants are possible but
not required for the language's basic operation; the specification
defines SPSC as the canonical mechanism.

#### 14.3.4 Wide-atomic optimization (optional)

On platforms with hardware support for 128-bit atomic operations
(x86_64 with `CMPXCHG16B`, ARM64 with `LDXP`/`STXP`), the
implementation may use in-place 128-bit atomic updates for `i128` and
`u128` cells rather than relying on the triple-buffer publish cycle.
This is an optimization, not a correctness requirement; the
triple-buffer mechanism provides correct semantics on all platforms.

Platforms without wide-atomic support (WebAssembly, ARM32, etc.) rely
exclusively on the triple-buffer mechanism. Programs using `i128` or
`u128` reactive cells on such platforms function correctly; they pay
the full per-publish cost for those cells.

#### 14.3.5 Extensible pools for dynamic-size types

Dynamic-size cell types (`Vec[T]`, `SmallVec[T; N]`, `RingBuf[T; N]`,
`string`, etc., per §13.12.4) cannot live directly in the fixed-size
cell slots. Their storage uses **extensible pools** alongside the
reactive state buffer.

For each dynamic-size type used in the program, the kernel allocates
a per-type pool. The reactive cell holds a fixed-size handle (one
`AtomicI64` slot, encoding a pool index plus version metadata); the
actual variable-size value lives in the pool.

**Pool mechanics:**

- Each pool is an arena of slots. Each slot holds one value of the
  pool's type, plus refcount metadata sufficient for triple-buffer
  rotation.
- Producer writes: when the kernel commits a new dynamic-size value
  for a cell, it allocates a fresh pool slot, writes the value, and
  publishes the new handle into the back buffer.
- Consumer reads: dereference the handle through the pool to obtain
  the value's address, then read the value.

**Triple-buffer interaction:**

Each of the three buffer copies independently references its own
pool slot for any given dynamic-size cell. When the producer
commits, the back buffer's handle is updated to a new slot; the
previous "current" buffer's handle still points at the old slot
until rotation reassigns its role.

For persistent data structures (e.g., `Vec[T]` as persistent vector
trie), pool slots may share internal nodes across versions. The pool
tracks the trie's node-level refcounts; old nodes are reclaimed when
no buffer references them.

For value types (`SmallVec[T; N]`, `RingBuf[T; N]`), each buffer's
slot holds a complete copy of the value. Rotation of the
triple-buffer ensures consumers never see partial writes; producer
work is bounded by the value's size.

**Initial allocation:**

Pool sizes are chosen at kernel construction based on graph specification
(static count of cells per type) plus a configurable headroom for
versioning. Pools may grow at runtime if the configured headroom is
exceeded; growth is amortized but not guaranteed wait-free. Hosts
needing strict wait-free guarantees should configure sufficient
headroom up front.

**Cost characteristics:**

- Allocation per commit: O(1) amortized for slot acquire (free-list
  in pool); O(value-size) for value copy. Persistent structures
  copy O(log n) nodes per push.
- Read: one pointer dereference through the pool.
- Memory: per-cell overhead is one handle slot (8 bytes); per-value
  overhead depends on the type. Persistent structures share storage
  across versions; flat structures replicate per buffer.

**Stream ring buffers** (§13.18) are a special case of pool-managed
allocation. Each stream declaration with element type `T` and
capacity `N` allocates a ring buffer of `N * sizeof(T)` bytes. All
stream instances sharing the same `(T, N)` combination across the
program draw from a per-`(T, N)` pool:

- The kernel enumerates stream declarations at compile time, groups
  them by `(T, N)`, and computes the per-pool size as the number of
  instances of that combination.
- Each pool slot holds one complete ring buffer. The stream's
  metadata cells (head pointer, dropped/rejected counters,
  observation cells per §13.18.6) live in the standard reactive
  state buffer; only the ring buffer's slot array lives in the
  per-`(T, N)` pool.
- Hot reload can grow or shrink these pools as stream declarations
  are added or removed, per the same extensible-pool mechanism. A
  preserved stream (per §13.18.11's preservation rule) retains its
  pool slot across reload; a new stream allocates a new slot.

Unlike persistent data structures, ring buffer slot arrays are not
shared across triple-buffer copies. The synchronization protocol
relies on the head pointer (which IS triple-buffered): producers
write to slot positions, then advance the head pointer at publish
time; consumers reading via swap observe events only up to the
head pointer committed by the most recent publish. Producer writes
to slot positions that haven't reached the committed head are
invisible to consumers until the next publish. Overwrites of
previously-committed slots (under `ring` policy) happen only at
positions past any cursor that's caught up; lagging cursors that
were pointing at overwritten positions jump forward per §13.18.9.

**Drop and eviction:** see §14.8.

### 14.4 What Lives in the Reactive State Buffer

Only **reactive cells** live in the triple-buffered reactive state
buffer. Specifically, the values held by:

- `signal` declarations.
- `attr` declarations on node and connection instances.
- `recurrent` declarations on node and connection instances.
- `derived` declarations (the cached computed value).
- `stream` declarations (head pointer, metadata, and synthesized
  observation cells per §13.18.6; the ring buffer slot array itself
  lives in the per-`(T, N)` pool per §14.3.5).
- Cells declared inside an `effect`'s `desired:` and `observed:`
  blocks (§13.19.4, §13.19.5). These are ordinary Signal or
  Stream cells per their declared type; the effect is a grouping
  construct, not a new storage category.

`Sink[T]` cells (§13.18.4) are the write-side view of a Stream;
they share the same underlying storage and do not allocate
separately.

Regular Ductus values — local bindings (`let`/`mut`) inside function
bodies, function parameters, function return values, iterator state,
closure captures, ordinary record/array/tuple values used as
non-reactive data — do **not** live in the reactive state buffer.
They are normal Rust values in stack or heap memory, governed by the
ownership and borrow rules of §11.

A record type may appear in both contexts in the same program. As the
value of a signal/attr/recurrent/derived declaration, it occupies cells
in the reactive buffer. As a local value, parameter, or non-reactive field,
it lives in regular memory. The Ductus compiler determines storage
location based on the declaration site, not the type.

### 14.5 Strings and the String Pool

Strings are variable-length and refcount-shared per §11.6. Their
storage requirements do not fit the fixed-size cell model of the
reactive buffer.

The kernel maintains a separate **string pool** that stores all string
content. The pool is logically a refcounted-shared, append-mostly
arena: each unique string is stored once and shared via reference
counts.

Reactive cells of type `string` store a **handle** (u64) into the pool
rather than the string content itself. The handle indexes the pool;
the pool resolves the handle to the actual `Arc<str>` data.

#### 14.5.1 Cross-thread consistency

The pool is shared across all three buffer copies. Buffer copies hold
handles; the pool holds the data. This separation allows:

- Buffer publish cost to remain O(N) in *cell count*, not in *string
  content size*. Changing a 1-megabyte string updates a single 8-byte
  handle in the buffer; the megabyte of data is allocated once in the
  pool, not three times in three buffer copies.

- Strings to be referenced by multiple cells (in the same or different
  buffers) via shared handles. Refcounting ensures the data is
  reclaimed when no buffer holds the handle.

#### 14.5.2 Pool operations

- **Allocation**: the producer (§14.7) allocates a new string in the pool;
  pool returns a handle. Refcount initialized to 1 for the cell that
  will hold it.
- **Refcount increment**: when a handle is copied into another cell,
  the pool's refcount on that string increments.
- **Refcount decrement**: when a cell is overwritten or buffer is
  retired, the previous handle's refcount decrements. If refcount
  reaches zero, the pool reclaims the string's storage.
- **Lookup**: consumer thread reads a handle from the buffer, looks
  up the corresponding `Arc<str>` in the pool (wait-free with proper
  pool structure).

The pool's allocation and refcount operations are atomic but may
block briefly under contention. These operations are performed by
the producer role (§14.7); the consumer role only reads via handles,
which is wait-free. The role-to-thread mapping is implementation-defined:
in typical native deployments the host's main thread plays the producer
role and one or more application threads play the consumer role; other
deployments may assign a kernel-configured thread to the producer role.
The mechanism (§14.3.3, §14.7) does not depend on the mapping choice.

### 14.6 The Behavior ABI

Each reactive behavior — a `derived` expression body or a `recurrent`
arm body — is exposed to the kernel via a uniform **behavior ABI**.
Functions called from reactive bodies are reactive-transparent per
§13.12.2: they compile to ordinary Rust functions (per §15.5) reached
transitively from the registered behaviors, not as separately-registered
behaviors of their own.

#### 14.6.1 Behavior signature

Every behavior has the same calling convention:

```
fn behavior(kernel: &KernelHandle, instance: InstanceId) -> ()
```

- `kernel`: a borrowed handle to the kernel, used for reading and
  writing reactive cells, allocating strings, and other kernel
  services.
- `instance`: an opaque identifier for the specific node or connection
  instance the behavior is being invoked for (relevant for `attr` and
  `derived` declarations on a particular instance).

The behavior reads its inputs from kernel cells via the handle,
performs its computation, and writes its outputs (if any) back to
kernel cells. Return value is unit; all effects are side effects
through the kernel handle.

This uniform shape means the kernel maintains a single function
pointer table: `Vec<fn(&KernelHandle, u64) -> ()>` (in the
reference Rust implementation; the function pointer type uses
`&KernelHandle` and relies on Rust's higher-rank trait bound
semantics for the lifetime parameter). The kernel invokes
behaviors by index into this table; no per-behavior dispatch logic
is needed.

`InstanceId` is a transparent newtype over `u64` defined in the
kernel; the function-pointer table uses `u64` directly since
`fn`-pointer types in Rust do not preserve newtype identity at the
ABI level. The two are interconvertible at zero cost.

#### 14.6.2 Statelessness

Behaviors are **stateless** at the kernel level. All state lives in
reactive cells (attrs, signals, derived results). Behaviors are pure
transformations: read inputs from cells, compute, write outputs.

A "stateful" computation (filter with sample history, oscillator with
phase, accumulator) is structured as a record whose attrs hold the
state, plus a behavior that reads the state-attrs, computes new
state, and writes back to the state-attrs.

Local mutation within a behavior (`mut` bindings, indexed assignment,
iterator state) is permitted per §§11–12. These mutations are visible
only within the behavior's invocation; they do not escape.

#### 14.6.3 Error handling

Behaviors follow Ductus's two-track failure model per §13.13:
trap-track failures (arithmetic overflow under default operators,
division by zero, out-of-range indices, explicit `panic`) abort the
process; recoverable conditions are expressed as value-track
`Option`/`Result` values flowing through the type system. The
kernel does not isolate behavior traps — there is no `catch_unwind`
boundary, no errored-cell sentinel, no continuation past a trap.
See §13.13.1 for full rules and worked examples; the same semantics
apply uniformly to all behaviors invoked by the producer.

#### 14.6.4 Behavior identity

Each behavior is identified by a stable u32 ID assigned at compile
time. IDs are **content-addressed**: a stable hash of the canonicalized
typed IR of the behavior body. "Canonicalized" means the IR is
normalized (alpha-renamed locals, sorted decl order where order is
irrelevant, position information stripped) before hashing, so
cosmetic changes — adding whitespace, reordering independent
declarations, renaming local bindings — do not perturb the ID.
Semantic changes — different operations, different inputs, different
output type — produce different IDs.

The hash algorithm is fixed per Ductus toolchain version (§14.10)
so that hot reload (§14.9) within one version reliably matches
unchanged behaviors across recompilations. Across major toolchain
versions the canonicalization may change; cross-version hot reload
is not supported.

Each behavior also carries a debug name: the qualified source path
(`module::path::clip_name::derived_name`). Names appear in
diagnostics, profiles, and error messages. The kernel resolves
behaviors by ID; debug names appear only in diagnostic output.

#### 14.6.5 Thread invocation

Behaviors are invoked by the kernel; the specific thread that
invokes each behavior is the producer-role thread, which the kernel
maps to a specific OS thread at startup (implementation-defined per
§14.7.1). Ductus source does not specify thread roles.

Ductus source code does not encounter cross-thread concerns:
behaviors are thread-safe by construction (no shared mutable state
outside reactive cells, which are coordinated by the kernel per
§14.3.3).

### 14.7 Producer and Consumer Roles

The triple-buffer mechanism (§14.3.3) operates in terms of two roles:

- **Producer**: the role that writes the back buffer, runs publish
  cycles (evaluation + atomic swap). There is exactly one producer
  per kernel instance (SPSC). The producer may also read the back
  buffer it is writing; such reads are local to the producer and
  do not go through the triple-buffer pointer swap. What the
  producer writes (signal/attr updates from host API, derived and
  recurrent arm expression results) and what triggers it to publish
  are specified in §13.10.
- **Consumer**: the role that reads the current buffer via the swap
  operation. Loads the current pointer and reads cells from the
  buffer it points to. Never writes; never invokes behaviors. There
  is one consumer per SPSC channel; if multiple cross-thread
  observers are needed, each maintains its own SPSC channel
  (§14.3.3.4).

§14 specifies only the mechanism of these roles — what each role is
permitted to do, how the two coordinate via the triple buffer, and
the costs of the swap and publish operations. The mapping of roles
to physical threads and the choreography of what the producer does
between publishes are implementation-defined; the trigger that
initiates a publish is specified in §13.10 (the kernel's evaluation
cycle).

#### 14.7.1 Thread-safety properties of the mechanism

By construction of the SPSC triple buffer:

- The producer writes the back buffer without interference; the
  consumer never touches it.
- The consumer reads the current buffer without interference; the
  producer never touches it.
- The atomic current-pointer swap is the synchronization point
  between producer and consumer.
- No locks are required, no spin-wait is required, and reads are
  wait-free.

These properties hold regardless of the role-to-thread mapping, which
is implementation-defined: in typical native deployments the host's
main thread plays the producer role and one or more application threads
play the consumer role; other deployments may assign a kernel-configured
thread to the producer role. The mechanism (§14.3.3, §14.7) does not
depend on the mapping choice.

#### 14.7.2 Behaviors invoked by the mechanism

Reactive behaviors (derived expression bodies and recurrent arm
expressions) are invoked by the producer. Functions called from
reactive contexts are reactive-transparent per §13.12.2 and reached
transitively from registered behaviors; they are not themselves
separately invoked by the producer. The trigger, the selection of
which behaviors are invoked, and the ordering of invocations within
a publish cycle are all specified in §13.10.

The behavior ABI (§14.6) is the contract between the producer and
each invoked behavior. Each invocation receives a kernel handle
and an instance ID; behavior bodies read from and write to cells
via the handle. Behaviors are thread-safe by construction
(§14.7.3).

#### 14.7.3 Why Ductus behaviors are thread-safe by construction

Regardless of the role-to-thread mapping (implementation-defined per
§14.7.1), Ductus source code never sees cross-thread concerns:

- No shared mutable state outside reactive cells.
- Reactive cells are coordinated through the triple-buffer
  mechanism above.
- Local `mut` bindings (§11) are stack-allocated and per-invocation.
- Closure captures are by-value Copy (§11.10), no shared mutability.

A Ductus program does not declare thread affinity; it does not
need to. The kernel determines (implementation-defined per §14.7.1)
which thread plays which role.

### 14.8 Drop Semantics

Ductus's `Drop` trait — referenced from §11.3.3 and §12.9.3 — is
specified here.

#### 14.8.1 The Drop trait

```
trait Drop:
  fn drop(value: mut Self)
```

A type implementing `Drop` provides cleanup logic that runs when a
value of the type goes out of scope. The `drop` method receives the
value by `mut` (the only place in the language where a `mut`
parameter is permitted — internally generated by the compiler at the
scope-exit point).

#### 14.8.2 When drop runs

The compiler inserts drop calls at:

- The end of a value's lexical scope (when its `let` or `mut` binding
  goes out of scope).
- The point of consumption (when a value is moved into a function
  parameter or assignment; the moved-out source's drop slot is empty
  thereafter).
- The end of a function for un-returned locals.
- The point of `break` for non-yielded iterator elements (§12.9.3).

Compound values (records, enums) drop in **reverse declaration
order** of their fields: the last-declared field drops first.

#### 14.8.3 Partial moves

If only some fields of a record have been moved out when the binding
goes out of scope, only the un-moved fields drop. The compiler tracks
per-binding move flags during semantic analysis.

#### 14.8.4 Drop and panic

If a `drop` method panics, the process aborts (the standard trap
behavior per §4.6.1). This prevents double-drop hazards from
mid-drop panics that would otherwise leave the program in an
inconsistent state.

#### 14.8.5 Drop on reactive cells

The kernel manages drop for reactive cells. When a node or connection
instance is removed (removal mechanics are specified in §13.15), its
attr and derived cells are dropped per their type's `Drop` impl.
Initial declarations (signals declared at program startup) live for the
program's lifetime; their cells are dropped at program shutdown.

#### 14.8.6 Drop and triple-buffer eviction for dynamic-size cells

Dynamic-size cells (per §13.12.4 and §14.3.5) require eviction
ordering across triple-buffer rotation. When the kernel commits a
new value for a dynamic-size cell, the previous value is still
referenced by the rotating-out buffer slot until rotation makes
that slot the next back buffer.

**Rotation rule:**

A pool slot for a dynamic-size cell becomes eligible for `drop`
when no buffer references its handle. Concretely:

1. Producer commits new value → new pool slot allocated → back
   buffer's handle updated to new slot.
2. Atomic swap → back becomes current; previous-current's handle
   still points at the old slot.
3. Consumer eventually reads the new current (catches up).
4. Next publish → previous-current rotates to back. At this point
   the back buffer's slot reference is replaced; the old slot is
   now unreferenced from any buffer.
5. The kernel runs `drop` on the old slot's value, then releases
   the slot to the pool's free list.

**For persistent data structures (e.g., `Vec[T]` as persistent
trie):** drop runs at the trie-node level rather than the whole-Vec
level. Internal nodes shared across versions remain alive until all
referencing versions have been evicted. The pool tracks per-node
refcounts; nodes drop when their refcount reaches zero.

**Drop ordering invariants:**

- A drop runs only after the value is unreferenced by any buffer.
- Drops run on the producer thread (or a dedicated reclamation
  thread), never on the consumer thread.
- Drops complete before the slot is reused — no in-place reuse of
  a slot whose drop hasn't finished.

The synchronization between rotation-out and drop+reclamation is
provided by the kernel's per-pool reclamation epoch: the slot enters
a quarantine state when its handle is replaced; the pool's
reclamation thread (or the producer thread, depending on
implementation) advances the epoch atomically and runs drops on
quarantined slots before releasing them to the free list. No drop
runs while any buffer still references the slot's handle.

**Drop and panic:** if `drop` panics on a dynamic-size cell value,
process abort applies per §14.8.4. The pool slot is leaked but the
process is terminating anyway.

### 14.9 Hot Reload

Interpreter mode (§14.1.2) supports hot reload of individual
behaviors in a running kernel.

#### 14.9.1 Granularity

The unit of hot reload is the **behavior**. When a Ductus source
file changes:

1. The CLI's watch mode detects the change.
2. The frontend re-runs on the changed file.
   3a. **Behavior identity (§14.6.4).** The frontend computes
   content-addressed IDs for each behavior per §14.6.4. Behaviors
   whose IDs are present only in the old program are *removed*;
   behaviors present only in the new program are *added*; behaviors
   present in both are *carried over* unchanged.
   3b. **Cell identity (§13.15.2).** The kernel computes the cell-diff
   by fully-qualified declaration path. Cells with matching path and
   type carry forward (preserving values); new cells are added;
   removed cells are dropped per §14.8.
   3c. **Operator instance identity (§13.17.10).** Operator instances
   are matched by (enclosing scope, operator name, argument bindings)
   with tolerance for positional moves within the same scope. Matched
   instances preserve their internal cell state via 3b; unmatched
   instances are dropped/added with the corresponding cell churn.
4. **Apply additions.**
    - For each added behavior: register in the behavior table at its
      content-addressed ID; graph specification edges and cell allocations
      referencing the new behavior's ID become live; subsequent
      invocations dispatch through the new behavior's ID.
    - For each added cell: allocate space in the reactive state buffer
      and initialize per the new source.
    - For each added operator instance: allocate internal cell state
      and initialize per the new source.

5. **Apply removals.**
    - For each removed behavior: deregister from the behavior table.
    - For each removed cell: invoke drop per §14.8 in
      reverse-declaration order.
    - For each removed operator instance: drop internal cells per
      §14.8.

6. **Run re-initialization evaluation pass.** For each derived whose
   behavior body changed (different content-addressed ID), recompute
   its initial value from current inputs. Deriveds whose body is
   unchanged retain their values.

7. **Publish the reloaded state** (atomic current-pointer swap).

8. **Release the reload lock.** Resume signal/attr writes; apply any
   queued writes to the new state.

#### 14.9.2 State preservation

Reactive cell values persist across hot reload. Signal values, attr
values, and derived cached values are unchanged unless the source
explicitly changes them. The graph topology persists.

Operator instance state is preserved across reload by the
operator-instance-identity scheme of §13.17.10 (matched by enclosing
scope, operator name, and argument bindings, with tolerance for
positional moves within the same scope). Matched instances preserve
their internal cell state via the same cell-identity mechanism
(§13.15.2) used for top-level cells.

#### 14.9.3 Reload-safe and reload-unsafe changes

Changes safe to hot reload:

- Body of an existing behavior (same signature, different
  implementation).
- Adding new behaviors (new derived expressions, new recurrent arm
  bodies).
- Adding new signals, attrs, derived declarations.

Changes unsafe for in-place hot reload fall into two classes per
§13.15.4:

- **Full-kernel restart** is required for changes to the reactive
  state buffer layout that would require relocating live cells.
- **Per-instance restart** is sufficient for operator-specific
  cases (operator signature changes, internal cell type changes
  per §13.17.10); only the affected operator instances are
  recreated, not the whole kernel.

All other changes — including cell removal (which the new source's
compile gate verifies is unreferenced), cell type changes (handled
via remove + add per §13.15.2), and connection topology changes
(handled via remove + add per §13.15.2) — are reload-safe and need
no restart.

The implementation diagnoses unsafe changes at reload time and
either rejects them (kernel keeps running old version) or applies
the appropriate restart — full-kernel or per-instance per §13.15.4
— cleanly. The choice is implementation-defined.

#### 14.9.4 Reload failure

If the new source fails to compile (parse error, type error,
ownership error), the reload is abandoned. The kernel keeps running
the previous version. The CLI surfaces the compilation error to the
user.

Hot reload never produces a kernel in an inconsistent state. Either
the old version continues running, or the new version is fully
applied, never a mix.

### 14.10 Versioning

Ductus's source format, graph specification format, behavior ABI, and
kernel build are versioned together. Each Ductus release is a
matched set:

- Ductus source format version.
- Graph specification schema version.
- Behavior ABI version.
- Kernel binary version.

Cross-version mixing is not supported. A Ductus program produced
by version X.Y compiles with and runs against the same X.Y
toolchain. Forward and backward compatibility across major version
boundaries are explicit, not implicit.

The exact syntactic form of the `@version` directive is reserved for
a future spec revision; v1 implementations are not required to
recognize it. Version metadata is recorded in the graph specification
header (§15.4); cross-version compatibility checks happen there.

---

*End of §14.*

---

## 15. Compilation Model

Ductus compilation transforms source files into executable form plus
the build-time artifacts the kernel consumes at startup. This section
specifies the semantic obligations of the compiler (§15.2), the
artifacts it produces (§15.3), the normative format of the reactive
graph specification (§15.4), the Ductus-to-Rust lowering rules
(§15.5), the two compilation modes (§15.6), the diff algorithm hot
reload depends on (§15.7), and what implementations must satisfy to
be conformant (§15.8).

Runtime concerns — cells, kernel mechanics, threading, drop, hot-
reload application — are the subject of §14.

### 15.1 Overview

The compiler ingests Ductus source files and emits two artifact
classes:

- **Executable code** — bytecode (interpreter mode, §14.1.2) or Rust
  source compiled by `rustc` (native mode, §14.1.3).
- **The reactive graph specification** — a build-time description of
  the program's reactive shape that the kernel consumes at startup
  and that hot reload diffs against (§15.4, §15.7).

Both artifacts share the same frontend (§14.1.1), which performs
name resolution, type checking, trait resolution, borrow checking,
monomorphization, and reactive-graph extraction (§15.2). Backends
fork only at the final lowering step (§15.5).

This section does not prescribe the compiler's internal IR shape.
Implementations may use any phase organization that satisfies the
obligations of §15.2.

### 15.2 Compilation Obligations

A conformant Ductus compiler performs the analyses below and rejects
programs that fail any of them. The analyses are listed as
obligations, not phases; the compiler may organize them into any
combination of passes as long as each program is accepted or rejected
according to the language semantics defined in §§1–13.

#### 15.2.1 Required analyses

| Obligation                  | Spec reference            |
|-----------------------------|----------------------------|
| Lexical and syntactic parse | `GRAMMAR.md`               |
| Name resolution             | §3.4, §10                  |
| Type checking               | §2, §4, §6, §7, §9         |
| Trait resolution            | §3.4.2, §3.7               |
| Borrow checking             | §11.9                      |
| Monomorphization            | §2.3                       |
| Reactive-graph extraction   | §13, §15.4                 |

Failure to satisfy any obligation is a compile error; the compiler
rejects the program before emitting any artifact.

#### 15.2.2 Ordering constraints

The obligations form a partial order by data dependency:

- Name resolution depends on parsing.
- Type checking depends on name resolution.
- Trait resolution depends on type checking.
- Borrow checking depends on type checking.
- Monomorphization depends on trait resolution and type checking.
- Reactive-graph extraction depends on monomorphization (cells in
  generic node types are extracted per-instantiation).

The compiler may interleave these obligations across passes as long
as the partial order is preserved. The spec does not prescribe a
particular IR layering or pass count. The reference implementation's
canonical layering (AST → HIR → MIR → backend) is non-normative.

#### 15.2.3 Diagnostics

Diagnostic quality, error message formatting, and recovery behavior
are implementation-defined. The spec requires only that ill-formed
programs be rejected and well-formed programs be accepted per the
semantics of §§1–13.

### 15.3 Compilation Artifacts

A successful compilation produces two artifact classes:

- **Executable code** — the per-mode artifact described in §15.5
  and §15.6.
- **Reactive graph specification** — the normative build-time
  description of the program's reactive shape, specified in §15.4.
  The specification carries all build-time metadata the kernel
  needs (behavior table, string pool seed, schema and format
  versions); see §15.4.1 for the complete field list.

Both backends produce the same graph specification; only the
executable-code form differs.

#### 15.3.1 Embedding and packaging

In **interpreter mode** (§14.1.2), both artifacts are held in memory
by the running kernel. Hot reload (§14.9) replaces them in place.

In **native mode** (§14.1.3), both artifacts are embedded in the
compiled binary, typically as data sections produced by Rust's
`include_bytes!` or analogous mechanism. At program startup the
kernel deserializes the graph specification and registers the
behavior table.

### 15.4 The Reactive Graph Specification

The reactive graph specification is the build-time artifact
describing the program's reactive shape. It is the **interop
boundary** between the compiler and the kernel, and between two
builds of the same program for hot reload (§15.7).

The specification is defined as an abstract data model (§15.4.1).
The **canonical serialization** is JSON (§15.4.2); implementations
may additionally support binary or in-memory representations for
performance, but the canonical JSON form is the cross-implementation
reference.

#### 15.4.1 Abstract data model

The specification is a structured record with the following fields.

**Cells.** A list of cell entries. Each cell entry contains:

- `id`: the cell's fully-qualified declaration path (§15.4.1.1).
- `type`: the cell's primitive type tag, per §4.1, plus the
  string-handle (§14.5) and dynamic-pool-handle (§14.3.5) types.
- `observability`: one of `cross_thread_snapshot`,
  `cross_thread_atomic`, or `confined` (§15.4.1.2).
- `cadence_hint` (optional): one of `realtime`, `bounded`, or `lazy`
  (§15.4.1.3).
- `initial_value` (optional): the compile-time initial value for
  reactive-safe initializers per §13.8.2.1.
- `size`, `alignment`: derived from `type`, recorded explicitly for
  cross-implementation interop.

**Connections.** A list of connection entries. Each:

- `from`: source instance's fully-qualified path.
- `to`: destination instance's fully-qualified path (or `null` for
  sink-side connections).
- `connection_type`: the connection's declared type.
- `attrs`: ordered list of `(name, value)` pairs (§13.8.4).
- `when` (optional): gate predicate, encoded as a behavior ID and
  its input-cell list.

**Derived dependency edges.** A list of `(derived_cell_id,
[input_cell_ids])` pairs. Used by the kernel for dirty-set
propagation and topological evaluation ordering.

**Recurrent trigger sets.** A list of `(recurrent_cell_id,
[trigger_cell_or_event_ids], where_guard?)` tuples, encoding the
arm semantics of §13.2.4.

**`when`-gates.** Per gated instance, the predicate expression in
compiled form (behavior ID per §14.6.4, plus input cell IDs the
predicate reads).

**Behavior table.** A list of `(behavior_id, debug_name,
input_cell_ids, output_cell_id?)` entries. Behavior IDs are
content-addressed per §14.6.4. The kernel binds IDs to function
pointers at program startup.

**Stream cells.** A list of stream cell entries. Each:

- `id`: the stream's fully-qualified declaration path.
- `element_type`: the element type tag (per §15.4.1's `type`
  encoding).
- `policy`: one of `ring` or `gate`.
- `capacity`: integer literal capacity.
- `source_dependencies`: the input cells the stream's source
  expression reads (used for dirty-set propagation when the
  source is a derived chain).
- `observation_cell_ids`: IDs of the synthesized observation cells
  (per §13.18.6) — `pending_count`, `pressure`, `is_full`,
  `dropped_total`, `rejected_total`, `last_overflow_at`.
- `reset_on_reload`: boolean, true if the stream carries the
  `@reset_on_reload` annotation.

A Sink declared in an effect's `desired:` block shares its cell ID
with the corresponding Stream view; the spec records a single
stream entry, with a flag indicating that the cell is exposed in
both views.

**Effect instances.** A list of effect instance entries. Each:

- `id`: the instance's fully-qualified path (the binding name or
  pipe-position site, encoded per §15.4.1.1).
- `effect_type_name`: the effect's declared type name (used to
  dispatch to the host's reconciler — see `reconciler_dependencies`
  below).
- `parameter_bindings`: list of `(parameter_name, source_cell_id |
  value_literal)` pairs.
- `desired_cell_ids`: IDs of the cells declared in the effect's
  `desired:` block for this instance.
- `observed_cell_ids`: IDs of the cells declared in the effect's
  `observed:` block for this instance.

**Reconciler dependencies.** A list of `(effect_type_name,
[concrete_type_parameters])` pairs naming reconciler-registration
keys the host must provide via `kernel.register_reconciler`
(§13.14.7) before the kernel can enter the live state. For non-
generic effects, the parameter list is empty; for generic effects,
each instantiation is a distinct key.

**String pool seed.** String literals used by the program,
pre-loaded into the pool at startup (§14.5).

**Schema version.** The Ductus toolchain version that produced the
specification, per §14.10.

**Format version.** The version of the abstract data model itself,
distinct from the toolchain version. Allows the schema to evolve
independently of the source language.

##### 15.4.1.1 Cell ID syntax

A cell ID is the cell's fully-qualified declaration path: a
dot-separated sequence of identifiers naming the lexical nesting
from module root through enclosing instances to the cell name.

Example: `audio.synth_a.osc_1.frequency` — module `audio`,
top-level instance `synth_a`, nested part `osc_1`, attr
`frequency`.

The path is derived deterministically from source: nesting plus
declared instance/cell names. The syntax is identical to that of
§13.15.2 (cell identity across reloads); the two are the same
mechanism: §13.15.2 specifies hot-reload identity in source-level
terms, §15.4.1.1 specifies the wire format.

For anonymous or duplicated sibling placements (rare; the language
encourages explicit naming per §13.8), the compiler appends an
ordinal suffix `:N` where N is the declaration-order index among
siblings of the same type at the same nesting depth (zero-based).

Cell IDs are stable across the same source compiled by any
conformant compiler. Cross-implementation hot reload at the same
source version yields matching cell IDs by construction.

##### 15.4.1.2 Observability class

The `observability` field declares what concurrency contract the
cell must satisfy. The kernel selects a storage mechanism that
honors the contract.

| Value                    | Contract                                         |
|--------------------------|--------------------------------------------------|
| `cross_thread_snapshot`  | Multi-thread readers see a snapshot-consistent view; cross-cell consistency within one publish transaction; no torn reads. |
| `cross_thread_atomic`    | Multi-thread readers see single-cell atomic reads; no cross-cell consistency guarantee. Cell value must fit in one 64-bit atomic slot. |
| `confined`               | Cell accessed only from one thread; no atomic required. |

The mapping from observability to kernel storage is a runtime
concern, not a spec mandate. Typical mappings on a conformant
kernel:

| `observability`         | Typical mechanism      |
|-------------------------|------------------------|
| `cross_thread_snapshot` | Triple-buffer (§14.3.3) |
| `cross_thread_atomic`   | AtomicBuffer            |
| `confined`              | Plain memory            |

Alternative kernels may select different mechanisms as long as the
observability contract is met.

##### 15.4.1.3 Cadence hint

The `cadence_hint` field, when present, tells the kernel about the
update-timing expectation for the cell. It is informational; the
kernel uses it to bias storage-mechanism selection. Defined values:

- `realtime` — updates are deadline-bound; readers (e.g., audio
  thread) cannot block. Typically pairs with `cross_thread_snapshot`
  and selects a triple-buffer mapping.
- `bounded` — updates are committed but not deadline-bound. Pairs
  with any `observability` value.
- `lazy` — updates are best-effort; the cell tolerates large
  staleness. Typically pairs with `confined` and selects plain
  memory.

A cell without a `cadence_hint` is treated as `bounded`.

##### 15.4.1.4 Determining observability class

The compiler assigns each cell an observability class based on its
declaration kind and access pattern observed during reactive
analysis (§14.1.1 step 4). Defaults:

| Declaration kind                              | Default `observability`     |
|-----------------------------------------------|------------------------------|
| `signal` (top-level)                          | `cross_thread_snapshot`     |
| `attr` on a node/connection instance          | `cross_thread_snapshot`     |
| `recurrent` on a node/connection instance     | `cross_thread_snapshot`     |
| `derived` reactive cell                       | `cross_thread_snapshot`     |
| `stream` cell (head pointer + metadata)       | `cross_thread_snapshot`     |
| Stream observation cells (§13.18.6)           | `cross_thread_snapshot`     |
| Effect `observed:` cells (host-written)       | `cross_thread_snapshot`     |
| Effect `desired:` cells (program-written)     | `cross_thread_snapshot`     |
| Stdlib single-cell types per §13.12.4         | `cross_thread_atomic`       |
| Local `let`/`mut` inside a function body      | not in the graph spec (§14.4 — non-reactive) |
| Closure captures, function parameters         | not in the graph spec       |

The compiler may downgrade `cross_thread_snapshot` to `confined` for
cells provably accessed from only one thread (e.g., a `derived` on a
node instance whose enclosing graph context is single-threaded).
This optimization is implementation-defined. The compiler may not
upgrade observability — a `confined` cell becoming
`cross_thread_snapshot` would silently change concurrency semantics.

The default `cadence_hint` follows from the declaration's enclosing
graph context: cells declared inside placements that participate in
the kernel's evaluation cycle (§13.10) get `realtime`; cells on
non-realtime paths get `bounded`.

#### 15.4.2 Canonical serialization

The canonical serialization is JSON.

A conformant compiler produces graph specifications in JSON form,
conforming to a normative JSON Schema published alongside this
specification (`graph-spec.schema.json`, schema version per
§15.4.3).

Layout requirements for canonical JSON:

- Two-space indent.
- Object keys ordered as specified in §15.4.1 (field order is
  normative).
- Arrays in declaration order (cells in source order; behaviors in
  lexicographic order of content-addressed ID; exact ordering rules
  in the JSON Schema).
- UTF-8 encoding, no BOM, Unix line endings.

These layout rules make canonical JSON diff-friendly: two builds of
equivalent source produce byte-identical canonical JSON.

Implementations may additionally produce:

- **Binary serializations** (e.g., FlatBuffers, Cap'n Proto, custom
  bit-packed) for fast startup loading. The reference kernel
  supports a binary form for native-mode embedding.
- **In-memory representations** (e.g., direct Rust structs) for
  interpreter mode.

Cross-implementation interop requires that the canonical JSON form
be readable by all conformant kernels.

#### 15.4.3 Versioning

The specification carries two version numbers:

- **Schema version** — the Ductus toolchain version that produced
  the spec, per §14.10.
- **Format version** — the version of the abstract data model and
  JSON Schema themselves.

A conformant kernel accepts specifications whose format version it
understands. Format-version mismatches are diagnosed at load time
per §14.10.

#### 15.4.4 What the specification does not contain

The specification is type-erased at the kernel boundary. It contains
primitive type tags and cell layouts, but **not** Ductus's full type
system — no record definitions, trait conformances, or generic
parameters. These are compile-time artifacts of the frontend, fully
resolved before the specification is emitted.

The kernel's view of the program is: a graph of cells with primitive
types, dependency edges, behavior references, and gate predicates.
It does not need to understand records as records or traits as
traits; it manages bits in cells and invokes functions by ID.

### 15.5 Lowering (Ductus → Rust)

The native-mode Rust emitter (§14.1.3) lowers the typed IR to Rust
source per the rules below. Interpreter-mode bytecode emission is
implementation-defined and out of scope for this section.

#### 15.5.1 Type lowering

| Ductus                 | Rust                                                        |
|------------------------|-------------------------------------------------------------|
| `i8`–`i64`, `u8`–`u64` | Same Rust types.                                            |
| `i128`, `u128`         | Same Rust types (on supporting targets).                    |
| `f32`, `f64`           | Same Rust types.                                            |
| `bool`, `char`         | `bool`, `char`.                                             |
| `string`               | A newtype wrapping a kernel string handle (see §15.5.1.1).  |
| Tuples                 | Rust tuples.                                                |
| Arrays `T[N]`          | Rust arrays `[T; N]`.                                       |
| Records                | Rust structs with same field order.                         |
| Enums                  | Rust enums with same variant order.                         |
| Newtypes (§6.3)        | Rust newtype structs.                                       |

##### 15.5.1.1 String storage uniformity

The `string` type lowers to the same Rust representation regardless
of whether the binding is reactive or non-reactive: a newtype around
a u64 handle into the kernel's string pool (§14.5).

Reactive context (signal/attr/recurrent/derived value of type
`string`): the handle lives in a reactive cell. The pool entry's
refcount tracks how many cells reference the string across all
buffer copies.

Non-reactive context (local `let s = "hello"`, function parameter,
record field outside reactive declaration): the handle lives in
ordinary Rust memory. The pool entry is still refcounted; ownership
of the handle increments the refcount, dropping the handle (per
§14.8) decrements it. Strings created in non-reactive scopes are
reclaimed when their last handle is dropped — typically when the
function returns and locals go out of scope.

This uniformity means: all `string` values share one storage backend
(the kernel pool), regardless of where their handles are held.
There is no separate "Rust-local string" representation distinct
from the "kernel string" representation; the only difference is
*where the handle is stored* (cell vs ordinary memory), not what
the handle points to.

The §11.6 "refcount-shared immutable backing" model maps directly
onto the kernel pool. The pool *is* the shared backing.

#### 15.5.2 Function and trait lowering

Ductus resolves all generic instantiations and trait dispatch during
frontend processing (§14.1.1). Emitted Rust is fully monomorphic and
trait-free:

- A generic Ductus function `fn f[T](...)` becomes multiple
  monomorphic Rust functions, one per instantiation: `f_i32`,
  `f_f64`, etc.
- Trait method calls dispatch in Ductus to a specific function; the
  emitted Rust call is direct, not through a trait.
- Ductus traits are not declared in the emitted Rust. No `trait` or
  `impl` blocks appear (with the exception below).
- Operator overloading on Ductus numeric primitives uses Rust's
  built-in operators (`+`, `-`, etc.) directly; no trait emission
  needed.

The one exception: when a Ductus record overloads a Ductus operator
(e.g., a user-defined `Vec3` with `Add`), the emitter generates an
explicit `impl std::ops::Add for Vec3` block in Rust so that `+`
works on the type at the Rust level. This is a narrow mechanical
emission, not a full trait export.

#### 15.5.3 Ownership lowering

Ductus's ownership rules map directly to Rust's:

| Ductus                  | Rust                     |
|-------------------------|--------------------------|
| `let x = e`             | `let x = e;`             |
| `mut x = e`             | `let mut x = e;`         |
| Pass by value (move)    | Pass by value (move).    |
| `&T` parameter (borrow) | `&T` parameter.          |
| `for x in v:` (consume) | `for x in v` (consumes). |
| `for x in &v:` (borrow) | `for x in &v` (borrows). |
| `Copy` types            | `Copy` trait derived.    |
| `Clone`                 | `Clone` trait derived.   |

Ductus's `&v` form in for-loops compiles to Rust's `&v`. Ductus's
parameter borrow `&T` compiles to Rust's `&T`. Rust's borrow checker
enforces the same rules that Ductus's frontend already verified; any
code that passed Ductus's checks passes Rust's.

#### 15.5.4 Iterator lowering

Ductus's `Iterator` trait (§12.7) has signature `fn next(iter: Self)
-> (Option[Item], Self)`. Rust's standard `Iterator` trait has
signature `fn next(&mut self) -> Option<Item>`.

The Ductus emitter generates Rust code using Rust's `Iterator`
pattern internally for performance, while Ductus source code
continues to see the tuple-return form. The translation is
mechanical: each Ductus iterator implementation lowers to a Rust
struct with a `next(&mut self) -> Option<Item>` method, plus a
wrapper that exposes the tuple-return form for Ductus-internal use
during compilation.

The final emitted Rust module contains only the `&mut self` form.
The tuple-return wrapper exists in Ductus's IR during lowering for
type-system consistency with the source-level Iterator trait, and is
eliminated before Rust code generation. No runtime overhead from the
dual representation reaches the emitted binary.

This translation is invisible to Ductus source code. Ductus users
never see `&mut` in their code or in error messages.

#### 15.5.5 Reactive primitive lowering

Ductus's `signal`, `attr`, `recurrent`, and `derived` declarations
do not lower to Rust types directly. They lower to:

- Cell allocations in the kernel state buffer, described in the
  graph specification (§15.4).
- Behavior registrations (the body of a `derived` expression OR the
  body of a `recurrent` arm becomes a Rust function matching the
  behavior ABI, §14.6).
- Dependency edges in the graph specification.

The lowered Rust code contains no syntactic trace of `signal` /
`attr` / `recurrent` / `derived` keywords. They are pure
graph-construction directives, encoded into the graph specification
and behavior table.

### 15.6 Compilation Modes

The compiler supports two output modes, described in §14.1.2
(interpreter) and §14.1.3 (native). Both modes share the entire
frontend (§14.1.1) and produce identical graph specifications
(§15.4); they differ only in the executable-code artifact.

This section does not specify the bytecode format (interpreter mode)
or the per-mode build pipeline. Mode selection and toolchain
integration are implementation concerns documented in §14.2.

### 15.7 Hot-Reload Diff

Hot reload (§14.9) operates by comparing the graph specifications
(§15.4) of two builds of the same program: the currently running
build (`old_spec`) and the newly compiled build (`new_spec`). The
diff algorithm computes the changes the kernel applies.

This section specifies the diff algorithm and its result format. The
kernel's mechanics for applying the diff are §14.9; the source-level
identity rules the diff implements are §13.15.

#### 15.7.1 Diff algorithm

The diff is computed entry-by-entry across each artifact field of
the graph specification:

- **Cells.** Matched by `id` (the fully-qualified declaration path
  of §15.4.1.1, identical to §13.15.2). Outcomes:
    - Same `id`, same `type` → **carried over** (kernel preserves cell
      value).
    - Same `id`, different `type` → **changed** (drop + re-allocate;
      initial value from `new_spec`).
    - `id` in `old_spec` only → **removed** (kernel drops per §14.8).
    - `id` in `new_spec` only → **added** (kernel allocates and
      initializes).

- **Connections.** Matched by `(from, to, connection_type)`. Diff
  semantics as for cells: matched → carried over; otherwise removed
    + added. Attr value changes on a matched connection are value
      changes, not identity changes; the connection itself is carried
      over and its attrs updated per the kernel mechanics of §14.9.

- **Behaviors.** Matched by content-addressed `behavior_id` per
  §14.6.4. Same ID → carried over (no rebinding needed). Different
  ID → removed + added (kernel rebinds function-pointer table).

- **Derived dependency edges, recurrent trigger sets, when-gates.**
  Set diff by their respective keys (derived cell ID, recurrent cell
  ID, gated instance ID).

#### 15.7.2 Reload classification

The diff classifies the overall change set per §13.15.4 into one of
three categories:

- **Reload-safe** — applied in place per §14.9 (hot reload).
- **Per-instance restart required** — operator-specific reinit per
  §13.17.10.
- **Full-kernel restart required** — buffer-layout relocation per
  §13.15.4.

The classification is computed from the diff alone; the kernel does
not need to re-parse source.

#### 15.7.3 Diff result format

The diff produces a **reload plan**: a sequenced list of (cell
add/remove/change, connection add/remove/change, behavior
add/remove) operations the kernel applies in topological order.

The plan format is implementation-defined but must preserve the
ordering constraints of §14.8 (drop reverse-declaration order;
connections before endpoint instances; etc.) and §14.9.

### 15.8 Conformance

A Ductus implementation consists of a **compiler** and a **kernel**
that operate on the same graph specification format. An
implementation is conformant if both components meet their
respective criteria.

#### 15.8.1 Compiler conformance

A conformant compiler:

1. Accepts every program that the language semantics of §§1–13
   define as well-formed.
2. Rejects every program that the language semantics define as
   ill-formed (with diagnostics; format is implementation-defined
   per §15.2.3).
3. Produces a reactive graph specification conforming to the
   abstract data model of §15.4.1, serializable in the canonical
   JSON form of §15.4.2.
4. Produces executable code that, when run against a conformant
   kernel with the produced graph specification, exhibits the
   observable behavior defined by §§1–13.

#### 15.8.2 Kernel conformance

A conformant kernel:

1. Accepts any reactive graph specification conforming to the
   abstract data model of §15.4.1 (in canonical JSON form or any
   other format the kernel additionally supports).
2. Allocates cells per the observability and cadence contracts of
   §15.4.1, using any mechanism satisfying those contracts.
3. Implements the runtime semantics of §13 and §14: cell evaluation
   order, drop semantics, hot reload, thread orchestration.

#### 15.8.3 Interoperability

A conformant compiler's canonical-JSON graph specification must be
loadable by any conformant kernel at the same format version.
Cross-implementation mixing (compiler from implementation A, kernel
from implementation B) is permitted at the same schema and format
version per §15.4.3.

#### 15.8.4 Conformance testing

The spec does not prescribe a reference test suite. Implementations
may publish conformance suites; passing such a suite is not a
normative requirement.

---
