Type System Topic 1: Placeholder is compile-time only, propagates through names, resolves at use sites.
The placeholder numeric type exists solely during compilation and never survives to codegen. Every runtime value has a
concrete machine type (i32, f64, etc.). This preserves predictable performance, sound typing, and known memory layout,
and it composes directly with monomorphization.
A name bound to a placeholder-typed expression itself carries the placeholder forward; each use site resolves
independently based on local context. A name bound to a concrete-typed expression carries that concrete type. The
placeholder is a property of values, not of bindings — bindings are transparent aliases.
Consequences: unused bindings need no resolution (dead code). Single-use bindings resolve at their one use site.
Multi-use bindings with placeholder values resolve per-site, so different sites can yield different concrete types from
the same source name without conflict. No special ambiguity error exists at the binding level; the only errors come from
use sites that are themselves ambiguous in isolation, which is a property of the site, not the binding.
Rejected: first-class runtime placeholders (drifts toward dynamic typing, unpredictable layout, hidden dispatch costs)
and binding-level ambiguity errors (unnecessary once placeholders propagate through names).

Topic 2: Body typechecking is definition-time with inferred constraints.
Generic function bodies are typechecked at definition, not deferred to call sites. The compiler infers required
capabilities from the operations the body performs (e.g., lerp requires its parameters to support +, -, *) and those
become constraints on the generic signature. Call sites must satisfy the inferred constraints.
This gives error locality (bugs point at the definition, not at call sites), isolated verification (generic functions
are valid before any call exists), working tooling for uncalled generics, and self-documenting signatures via inferred
constraints. The trait/capability mechanism required to express constraints is already a committed part of the language
design.
Rejected: instantiation-time-only checking (C++/Zig style). Its "flexibility" is mostly illusory for numeric generics,
and the costs — call-site error messages for definition bugs, no isolated verification, broken tooling on uncalled
code — are real.

Topic 3: Nominal, user-definable traits with auto-implementations for built-in numeric types.
Traits are nominal (Rust-style): a type satisfies a trait only via an explicit implementation. This preserves error
quality, coherence, and semantic meaning of traits, and prevents accidental conformance.
The language ships with a set of built-in numeric traits (Numeric, Integer, Float, Ord, etc.) pre-implemented for all
built-in numeric types. Users write no boilerplate to use numeric generics on built-in types. Users define their own
traits and implement them explicitly for their own types using the same mechanism.
Inferred constraints from generic function bodies (Topic 2) reference these traits — e.g., lerp infers a constraint on
the appropriate numeric trait based on the operations it uses.
Rejected: built-in-capabilities-only (creates a two-tier system, doesn't scale to user types) and structural/inferred
traits (accidental conformance, weaker error messages, erodes trait semantics).
Deferred refinement: exact trait hierarchy shape (which traits subsume which) — a tuning decision, not a fundamental
one. Topic 4: Defaulting rules via trait-level default annotations.
Traits may declare a default concrete type as part of their definition. Built-in numeric traits declare appropriate
defaults (e.g., Integer → i32, Float → f64). User-defined traits may declare their own defaults using the same
mechanism. The declared default must itself satisfy the trait, compiler-enforced.
Defaulting is last-resort, per-use-site: the compiler resolves from local context first, and defaulting fires only when
a use site is constrained solely by a trait (or traits) with declared defaults and nothing else pins the type. Each use
site defaults independently — no global resolution.
When a use site is constrained by multiple traits with declared defaults, the most-specific trait in the hierarchy
wins (e.g., Float's default beats Numeric's because Float: Numeric). This depends on the trait hierarchy shape deferred
from Topic 3.
A trait without a declared default produces a hard error at any use site that would require defaulting through it ("no
default available for trait X"). This treats missing defaults as deliberate — some traits are too domain-specific to
pick a default for — and points the user precisely at the fix: annotate, or declare a default on the trait.
Trait-declared defaults are the only defaulting knob. Use-site overrides happen through annotation, not through
alternative defaulting mechanisms, preserving the principle that defaults are discoverable at the trait declaration.
Rejected: no defaulting (Option A — too much friction on common patterns); fixed compiler-internal defaults (Option B —
invisible, two-tier between built-ins and user code, doesn't scale); proximity-based context-sensitive defaults (Option
C — unpredictable, fragile to edits). Topic 5: Operator semantics across numeric kinds.
Operators with a clear mathematical meaning across kinds work on both and promote on mixed inputs. Operators that are
kind-specific in their meaning are restricted to the kind they make sense for. The unifying principle: explicit
semantics at the operator's definition, no silent truncation, no hidden coercions beyond the documented promotion rule.
Specific operators:

* / accepts Numeric operands (integer, float, or mixed) and always produces Float. Mathematical division, divorced from
  machine representation.
* // accepts Integer operands and produces Integer. Truncating integer division.
* % accepts both kinds. Result kind matches inputs; mixed kinds promote to float, parallel to arithmetic operator
  promotion.
* Bitwise (&, |, ^, ~, <<, >>) are Integer-only. Float operands are a type error. Bit-level operations on floats require
  an explicit reinterpret cast.
* Right shift (>>) is a single operator; behavior (arithmetic vs logical) is determined by the signedness of the operand
  type via the Integer trait impl. No separate >>> operator.
* Comparison (<, <=, >, >=) works on both kinds via an Ord-style trait. Mixed-kind comparison promotes integer to float.
  Float comparison follows IEEE 754 semantics including NaN behavior.
* Equality (==, !=) works on both kinds, mixed-kind permitted via the same promotion rule. Float equality is provided
  despite precision hazards; the alternative is more paternalistic than warranted and breaks legitimate uses (NaN
  checks, exact-zero checks, exact-value comparisons).
* Unary minus (-x) works on signed Integer and Float. Unsigned Integer operands are a type error — silent wrapping on
  negation is rejected as a footgun source.
  Mixed-kind promotion rule (used by +, -, *, %, comparisons, equality, and implicitly by /'s always-float result): if
  either operand is float, the operation proceeds in float; otherwise integer. Promotion is a defined, type-visible
  operation, not a silent coercion — monomorphization picks concrete types and emits the conversion explicitly at
  codegen.
  Trait constraints inferred from generic function bodies (Topic 2) reflect these operators: / infers Numeric on
  operands and Float on result; // infers Integer; bitwise ops infer Integer; arithmetic and comparison infer Numeric or
  Ord as appropriate.
  Rejected: single / with type-driven truncation behavior (C-style — produces the canonical numeric footgun); separate /
  and div requiring matched-kind operands at the call site (Option B — forces explicit kind decisions where mixed-kind
  math is the natural intent); float-equality removal (paternalistic, breaks legitimate use cases); silent wrapping on
  unsigned unary minus (subtle, bug-producing). Topic 6: Implicit widening conversions, lossless only.
  The type system performs implicit widening conversions when and only when the conversion is provably lossless — no
  value is changed, only representation. All other conversions (narrowing, signed/unsigned crossing, lossy float-to-int
  or int-to-float, and any conversion that could lose information) require an explicit cast.
  Lossless conversion rules:
* Integer to wider integer of same signedness: implicit. (i8 → i32, u16 → u64.)
* Unsigned to wider signed: implicit. (u8 → i16, u32 → i64.) Always representable.
* Signed to wider unsigned: explicit cast. Negatives don't fit.
* Same-width signed/unsigned crossing: explicit cast, either direction.
* Float to wider float: implicit. (f32 → f64.)
* Integer to float, exact-representable cases: implicit. (i8/u8/i16/u16 → f32; i32/u32 → f64.)
* Integer to float, precision-losing cases: explicit cast. (i32 → f32 because f32 has a 24-bit mantissa; i64 → f64
  because f64 has a 53-bit mantissa — but see pragmatic exception below.)
* Float to integer: explicit cast, always. Fractional part is lost.
  Pragmatic exception for i64/u64 in mixed-kind arithmetic with f64: implicit widening is permitted despite the formal
  precision hazard for values above 2^53. Forcing explicit casts on every common i64 + f64 expression is more friction
  than the bounded, rare hazard justifies. The precision behavior is documented; users handling very large integer
  magnitudes in float contexts are expected to be aware.
  Implicit widening is the mechanism behind Topic 5's mixed-kind promotion rule. When an expression mixes operand types,
  the narrower operand widens implicitly per these rules before the operation proceeds. Monomorphization emits the
  widening as an explicit conversion instruction at codegen — no hidden runtime cost beyond the conversion itself.
  Interaction with placeholders (Topic 1): placeholders being resolved at a use site resolve directly to the demanded
  concrete type rather than passing through an intermediate widening step. Widening fires on already-concrete values
  flowing between fixed-type contexts.
  Rejected: no implicit conversions (Option A — too much annotation noise for real numeric code, which mixes widths
  constantly); implicit narrowing or lossy conversions (Option C — silent data loss, the canonical second C footgun
  after integer division truncation); strict variant requiring explicit cast on i64/f64 mixing (more friction than the
  bounded hazard justifies for common numeric patterns). Topic 7: Annotation syntax.
  Variable annotations use postfix colon-type: let x: i32 = 5. The name comes first, the type is auxiliary, the value
  follows. Reads left-to-right, matches inference's primary-name model, scales to function signatures.
  Function signatures use fn keyword with postfix colon-type on parameters and arrow-return-type, both optional:
* Fully explicit: fn lerp(a: f64, b: f64, c: f64) -> f64
* Mixed: fn lerp(a, b, c) -> f64
* Fully inferred: fn lerp(a, b, c)
  Omitted parameter or return types are placeholders, resolved per Topic 1. Generic-by-default falls out of omission; no
  separate generic-parameter syntax is needed for the common numeric-generic case.
  Literal suffixes use underscore-separated form: 5_i32, 3.14_f64, 1_000_000_u32. Visually distinct, no parsing
  ambiguity, composes with the numeric-separator convention.
  Explicit casts use the as keyword postfix: x as i32, result as f64, big_int as u8. Required for all non-lossless
  conversions per Topic 6. Chains cleanly: x as i64 as f64. Uniform syntax for both infallible (widening) and fallible (
  narrowing, float-to-int, signed/unsigned crossing) conversions — the user knows from the type pair whether information
  could be lost.
  Trait names are valid type annotations and denote a constrained placeholder. let x: numeric = 5 means x carries a
  placeholder constrained to the numeric trait, resolved per use-site per Topic 1. Trait names are lowercase. This gives
  users a way to narrow an inferred constraint without committing to a concrete type — useful for documentation and for
  guiding inference.
  Rejected: prefix-type variable syntax (C-family inertia, reads poorly with inference); expression-level annotation (
  verbose for the common case); function-call cast syntax (conflicts with constructor semantics, wrong mental model for
  reinterpretations). Topic 7e (revised) and naming conventions across the language.
  Three distinct categories with three distinct conventions, settled as follows:
  Concrete primitive type keywords are lowercase, built into the language as reserved tokens. The set: i8, i16, i32,
  i64, i128, u8, u16, u32, u64, u128, usize, isize, f32, f64, bool, char. These are not names in any namespace; they are
  language tokens. Same convention as Rust and most modern systems languages.
  Built-in numeric placeholder keywords are lowercase, also built into the language as reserved tokens. The set:
  numeric, integer, float, signed, unsigned. Each is a language-level keyword that refers to "a placeholder constrained
  by the corresponding built-in trait." They are syntactic sugar at the language level — the language designer's
  decision to make these specific traits first-class for the placeholder system. They are not renamings of the traits;
  they are independent keywords that the compiler resolves to placeholder-constrained-by-trait at the type-checking
  layer.
  Trait declarations (built-in and user-defined alike) are PascalCase identifiers living in the trait namespace. The
  built-in numeric traits are Numeric, Integer, Float, Signed, Unsigned, plus the fine-grained operator traits (Add,
  Sub, Mul, Div, Neg, Rem, IntDiv, WrappingAdd, SaturatingAdd, CheckedAdd, and so on per Topic 9 and Topic 10), plus
  standalone traits Ord and Eq. User-defined traits follow the same PascalCase convention. The language does not
  generate or reserve lowercase keyword aliases for user-defined traits — those live entirely in user space and are
  referred to only by their PascalCase identifiers.
  The mapping between the lowercase keyword and the PascalCase trait is fixed by the language for the built-in numeric
  placeholder keywords:
* numeric → placeholder constrained by Numeric
* integer → placeholder constrained by Integer
* float → placeholder constrained by Float
* signed → placeholder constrained by Signed
* unsigned → placeholder constrained by Unsigned
  Usage examples:
  let x: numeric = 5 // placeholder annotation, lowercase keyword
  let y: i32 = 5 // concrete type, lowercase keyword
  fn lerp[T: Numeric](a, b, c: T) -> T // explicit generic bound, PascalCase trait
  fn sum[T: Numeric & Ord](xs)         // trait conjunction, PascalCase traits, & operator
  int and number from the prior grammar draft are dropped. The placeholder concept is covered entirely by the lowercase
  keywords above; no separate "general numeric" keyword exists. Removing them eliminates the three-tier confusion in the
  old grammar and aligns the language with the placeholder model from Topic 1.
  Rejected: PascalCase for the placeholder keywords (would conflate language-level keywords with namespaced trait
  identifiers); generating lowercase aliases for user-defined traits (renames things outside the language designer's
  scope, pollutes the keyword space, and adds no value since user code already uses the PascalCase trait name everywhere
  else); keeping int and number (redundant third tier — numeric, integer, float cover the same ground with cleaner
  semantics).

Topic 8: Monomorphization scope and code generation.
Instantiation granularity is strict structural: each unique tuple of concrete types at a call site produces a distinct
instantiation. lerp(i32, i32, i32) and lerp(i32, i32, f64) are separate instantiations even if they share most of their
structure. Backend may deduplicate identical machine code as an optimization invisible to language semantics, but
deduplication is not part of the semantic model.
Monomorphization is per-call-site across module boundaries. When a generic function defined in module A is called from
modules B and C with different concrete types, each calling module emits its own instantiation. Generic function bodies
must therefore be available to any module that calls them, in source form or in an intermediate representation rich
enough for instantiation. Generic definitions function as headers from the linker's perspective, not as closed binary
implementations. This is a real constraint on the module system design, flagged for later.
Polymorphic recursion is forbidden. A recursive call within a generic function body that would require a different type
instantiation than the caller is a compile error. Direct same-type recursion is fine — it reuses the same instantiation.
The cases where polymorphic recursion is genuinely needed (certain advanced data structures, functional patterns) are
out of scope for the numeric-focused design; explicit dynamic dispatch can be added later if these cases become
necessary.
Dead code elimination operates per-instantiation. Each monomorphized variant is independently eligible for DCE. A
generic function with no call sites produces no output at all. A generic function called with some type combinations but
not others produces exactly the instantiations called, nothing more. The semantic unit for codegen is the instantiation,
not the generic.
Trait-bounded generics resolve to direct method dispatch at instantiation time. When lerp is instantiated for f32, calls
to +, -, * resolve to f32's specific impls and become direct function calls in the emitted code. No vtables, no
indirection, no runtime dispatch overhead. Coherence enforced by the nominal trait system (Topic 3) guarantees
unambiguous resolution — one impl per (type, trait) pair.
Binary size cost is an accepted tradeoff. Heavily-generic numeric functions called with many type combinations produce
many code copies. For the numeric-focused use case this is generally acceptable (few types, small functions).
Mitigations exist as future optimizations or opt-ins: backend deduplication of identical machine code; outlining of
type-independent code into shared helpers; explicit dynamic dispatch via trait objects as a later opt-in feature. None
of these are part of the current decision; they are noted as available levers if binary size becomes a real constraint.
Rejected: looser semantic deduplication (belongs in the backend, not the language); centralized cross-module
instantiation (incompatible with per-call-site type resolution under separate compilation, would require type erasure);
permitting polymorphic recursion with implicit type-erased dispatch (conflicts with Topic 1's commitment to
compile-time-only placeholders); per-function DCE (no such thing as a non-instantiated generic in the output, so
per-instantiation is the natural unit). Topic 9: Trait hierarchy via fine-grained operator traits with umbrella
combinations.
The trait hierarchy uses a two-layer design.
The fine-grained layer defines one trait per operator or capability, each declaring exactly the method(s) for that
operation. Each built-in numeric operator from Topic 5 maps to a dedicated trait. The set includes: add, sub, mul, div,
neg, rem, intdiv (for //), bitand, bitor, bitxor, bitnot, shl, shr, ord, eq. Each trait declares only what it provides.
No diamond inheritance, no awkward placement decisions — every operator lives on the trait that defines it, one-to-one.
The umbrella layer defines coarse traits as combinations of fine-grained traits, for convenience and for declaring
defaults. Umbrellas: numeric (combining add + sub + mul + neg + zero + one), integer (combining numeric + rem + intdiv +
bitand + bitor + bitxor + bitnot + shl + shr), float (combining numeric + div + ...), signed (refining integer with
neg), unsigned (refining integer without neg). A type satisfies an umbrella by satisfying its component fine-grained
traits.
Defaults (Topic 4) are declared on the umbrella traits where semantically meaningful: numeric → i32, integer → i32,
signed → i32, unsigned → u32, float → f64. The most-specific-wins rule from Topic 4 operates on the umbrella layer.
Inferred constraints from generic function bodies (Topic 2) reference the fine-grained traits — that's the precision
monomorphization needs to resolve methods correctly. A body using + infers add; a body using + and * infers add + mul;
etc. The compiler may simplify inferred constraint sets to umbrella equivalents where unambiguous, for readability in
error messages and signatures.
Explicit annotations (Topic 7e) can use either layer. let x: numeric = 5 constrains x to the umbrella. let x: add = 5
would constrain x to just the add capability, allowing types that support addition but not, e.g., multiplication. Users
choose granularity based on intent.
Operator-to-trait mappings are unambiguous: unary - on unsigned is a type error because unsigned doesn't satisfy neg.
Bitwise ops on floats are type errors because float types don't implement bitand etc. // on floats is a type error
because float doesn't implement intdiv. Each Topic 5 decision falls out naturally from the fine-grained trait
membership.
Built-in numeric types auto-implement the appropriate fine-grained traits per Topic 3. Umbrella satisfaction follows
from the combinations. Users defining their own numeric-like types implement individual fine-grained traits; a derive
mechanism (later feature) can auto-generate impls from a coarser declaration.
ord and eq are standalone fine-grained traits, not part of any numeric umbrella, since non-numeric types may also be
ordered or compared. Built-in numeric types auto-implement both.
Rejected: flat-and-coarse hierarchy (Option A — too imprecise to express Topic 5's operator distinctions, especially
signed/unsigned and integer/float divergence); operator-centric hierarchy with diamond inheritance and negatable-style
patches (Option C as it evolved — accumulates structural complexity to handle cases that Option B handles naturally);
pure fine-grained without umbrellas (verbose for explicit bounds, no natural place to declare defaults). Topic 10:
Overflow and arithmetic safety semantics.
Default arithmetic operators (+, -, *, /, %, unary -) trap on overflow at runtime, in all build modes. No debug/release
semantic split. The performance cost (a branch per op, well-predicted on modern hardware) is accepted in exchange for
uniform semantics and safety in production.
Dedicated operator variants provide explicit alternative policies:

* Wrapping (modular two's complement): +%, -%, *%, /%, %%, unary -%.
* Saturating (clamp to type bounds): +|, -|, *|, /|, %|, unary -|.
* Checked (returns Option<T>): +?, -?, *?, /?, %?, unary -?.
  Each operator variant corresponds to its own fine-grained trait per Topic 9: add, wrapping_add, saturating_add,
  checked_add, and analogously for the other operators. Built-in numeric types auto-implement all variants. Umbrella
  traits (integer, float, etc.) compose them. Inferred constraints from generic function bodies (Topic 2) pick the trait
  matching the operator used.
  Compile-time constant overflow is always a compile error. The compiler evaluates constant expressions and rejects
  programs where a constant value provably doesn't fit its declared or inferred type. This applies regardless of which
  operator variant was used — const x: u8 = 200_u8 +% 100_u8 is still a compile error because the constant can be
  evaluated and the result is out of range.
  Float overflow follows IEEE 754: produces infinity or negative infinity, not a trap. NaN, signed zero, subnormals all
  per IEEE 754. Float operators don't have wrapping/saturating/checked variants because IEEE 754 semantics are the
  convention and don't have meaningful "overflow" beyond producing infinity. The ? variant could exist on floats to
  detect NaN/infinity production if useful — flag for later refinement.
  Integer division by zero traps at runtime. The ? variant returns None instead of trapping. No wrapping or saturating
  variants for division-by-zero — there's no sensible modular or clamping answer to "divide by zero."
  Explicit narrowing or signed/unsigned crossing casts (as) trap on out-of-range values at runtime. Alternative cast
  methods are provided for non-trapping conversions: wrapping_as, saturating_as, checked_as (returning Option<T>). Cast
  operators don't get the symbol variants because casts are already explicit operations with a visible target type.
  Rejected: undefined behavior on overflow (the C signed-arithmetic mistake, source of decades of security
  vulnerabilities and miscompilation); debug-traps/release-wraps split (Rust's approach — creates production-only bugs,
  makes debug and release semantically different, performance gap is small on modern hardware); methods-only for
  explicit variants (verbose at call sites where intent should be operator-visible); per-site policy with no default (
  Zig-ish — too much verbosity for common arithmetic).
  Ready for the next topic? Topic 11: Literal behavior and compile-time evaluation via Option D — compile-time-known
  propagation through pure expressions, broken only by signals and external I/O.
  A value is compile-time known if its defining expression is compile-time evaluable. Since all let bindings are
  immutable and all functions are pure, compile-time-knowability propagates mechanically through the expression graph:
  literals are compile-time known; arithmetic and constructions over compile-time-known operands are compile-time known;
  calls to pure functions with compile-time-known arguments are compile-time known.
  Reactivity breaks the propagation. Signals (signal declarations) are defined as values whose moment of evaluation
  matters and are therefore runtime by definition. Any expression that transitively involves a signal is reactive and
  runtime, including function calls whose arguments include a reactive value. External I/O and any future host-boundary
  call has the same effect.
  For a value in a context demanding a concrete type, the compiler checks the value against the type's range:
* Integer values require exact fit (no overflow, no signed/unsigned violation).
* Float literal values fit any float type, rounded to nearest representable. Integer values fit float types only when
  exactly representable in that float's mantissa (per Topic 6's lossless rules), otherwise explicit cast required.
* Out-of-range values are compile errors, pointing at the source of the value with the computed-value and target-type in
  the diagnostic.
  Negative integer literals are parsed as a single signed token for type checking: let x: i8 = -5 checks -5 against i8's
  range, not "apply unary - to literal 5". This avoids the surprise that -5 would otherwise hit Topic 5's "unary - on
  unsigned" rule when the target type happened to be unsigned (Topic 5's rule still applies for runtime values).
  The propagation makes dependent-ish typing fall out for free: let arr: i32[fib(10) + 1] is a valid type because fib(
    10)
        + 1 is compile-time evaluable. Compile-time array sizes, configuration-driven generics, and
          bit-width-parameterized types all become first-class without separate const machinery.
          The compiler tracks reactivity provenance through expressions to produce precise error messages — "value of x
          is
          reactive because it depends on signal mouse_position at line 14" rather than "x is not constant."
          Implementation details flagged but not part of the semantic decision: compile-time recursion depth limit (
          configurable, per most languages with const eval); compile-time floating-point evaluation must use the
          target's float
          format exactly so compile-time and runtime agree; compile-time evaluation cost limit (configurable, so a
          runaway pure
          function doesn't hang the compiler indefinitely).
          Rejected: literals-are-special with named values requiring casts (Position B/C — unnecessary asymmetry given
          purity
          and immutability); strict literal-only flexibility (Position A — too much annotation noise on every literal in
          a typed
          context); separate const vs let keywords (purity + immutability collapse the distinction — let covers both).
          Topic 12:
          Array and index types.
          Array length type is isize — signed, platform-sized. The signed choice avoids the length - 1 footgun (0 - 1 on
          an
          empty array under unsigned would either freeze the loop or trap, while under signed it yields -1 and 0..-1 is
          correctly empty). The platform-sized choice scales addressing capacity with the machine. The theoretical
          halving of
          addressable size from usize to isize is not a real constraint on any current or near-future hardware: isize::
          MAX on
          64-bit platforms is ~9.2 × 10^18 elements, far beyond any real array. Users needing the "must be non-negative"
          invariant for low-level work (allocation sizes, FFI) can use usize explicitly; array lengths defaulting to
          isize does
          not prevent that.
          Array index type is any integer, implicitly widened to isize for indexing per Topic 6 lossless-widening rules.
          All
          concrete integer types except u64 widen losslessly to isize; u64 indices require explicit cast since values
          above
          i64::MAX don't fit. Users write indexing expressions with whichever integer type is natural for their
          context —
          counter variables, sizes, computed offsets — and the compiler handles the widening.
          Bounds checking on arr[i] traps at runtime if i < 0 || i >= length, consistent with Topic 10's
          trap-on-out-of-range
          philosophy. When both the index and the length are compile-time known per Topic 11, bounds checking happens at
          compile
          time and produces a compile error on out-of-range — arr[10] on i32[5] is rejected by the compiler, not at
          runtime.
          Array type syntax is T[N] exclusively. There is no exposed canonical Array[T, N] form; the standard library's
          underlying array type is internal and not addressable by name in user code. T[N] is dedicated syntax for the
          array
          type, not sugar for a public generic. This matches how tuples are typically handled — dedicated syntax, no
          namespace-level type name. Multi-dimensional arrays parse left-to-right: T[N][M] is an M-element array of
          T[N].
          Zero-length arrays T[0] are valid types, useful for edge cases and generic code.
          The dynamic-sized vector type (heap-allocated, growable) is a standard library concern, not a language-level
          type. Its
          name and syntax (Vec[T], Vector[T], or whatever the standard library chooses) is outside this topic. Only
          fixed-size
          arrays receive dedicated language syntax.
          Resolution of T[args] in type position: the grammar's TypePostfixOp is uniformly [arg-list]. The typer
          interprets it
          based on the syntactic-context's expectations. For array-type construction (e.g., i32[5]), the typer
          constructs the
          array type directly. For generic instantiation (e.g., Vec[i32]), the typer instantiates the generic with the
          given
          type arguments. The disambiguation is by what the TypeAtom resolves to — primitive types and other non-generic
          types
          take array shorthand; generic types take instantiation.
          Rejected: usize for length (the theoretical addressing capacity gain is unreachable on real hardware, and the
          length -
          1 footgun is concrete and common — modern language design has converged on signed lengths for exactly this
          reason);
          dual canonical-and-shorthand syntax for array types (introduces inconsistency and an unused Array[T, N] form
          that the
          typer would have to recognize but users would not write); making indexing require a specific integer type (
          isize-only) (would force casts on every common loop counter, contrary to Topic 6's convenience goal);
          language-level
          dynamic arrays (collection types beyond fixed arrays belong in the standard library, not the language core).
          Topic 13:
          Special numeric operations.
          Operations beyond the core arithmetic operators (sqrt, sin, cos, tan, asin, acos, atan, atan2, ln, log2,
          log10, log,
          exp, exp2, abs, min, max, pow, floor, ceil, round, trunc, etc.) are defined as trait methods, following the
          fine-grained trait pattern from Topic 9.
          Invocation forms (three syntaxes, one definition):
* Method-call: x.sqrt(), a.min(b).
* Pipe-forward: x >> sqrt, a >> min: b per the grammar's x >> name: arg desugaring to name(x, arg).
* Free-function via trait-path: Sqrt::sqrt(x), Min::min(a, b). The free function is the trait method accessed through
  its fully-qualified path. With the trait imported into scope (use root::math::Sqrt), the bare form sqrt(x) resolves to
  Sqrt::sqrt(x). Resolution follows the same trait-method-resolution mechanism used elsewhere; this is not a separate
  definition.
  All three forms call the same underlying trait method. Users pick the form that reads best at the call site. Chaining
  and data-flow code reads naturally in pipe-forward form; receiver-oriented code reads naturally in method form;
  mathematical notation reads naturally in free-function form. The language does not privilege one form over the others.
  Trait placement, per Topic 9's fine-grained-with-umbrella pattern:
* Abs, Min, Max are on Numeric — meaningful for both integers and floats.
* Sqrt, Sin, Cos, Tan, Asin, Acos, Atan, Atan2, Ln, Log2, Log10, Log, Exp, Exp2, Floor, Ceil, Round, Trunc are on
  Float — meaningful only for floats (integers don't have meaningful sqrt without losing information; floor/ceil on
  integers are identity and not provided as separate operations).
* Pow splits into two traits: IntPow on Integer (integer base, integer exponent, integer result, traps on negative
  exponent) and FloatPow on Float (float base, any-numeric exponent, float result). The typer picks the right trait
  based on the receiver's type. The umbrella Integer includes IntPow; the umbrella Float includes FloatPow.
  Built-in numeric types auto-implement all applicable traits per Topic 3.
  Special-case semantics:
* abs on the minimum value of a signed integer (e.g., i32::MIN.abs()) traps on overflow per Topic 10's default. Methods
  wrapping_abs and saturating_abs provide the non-trapping variants; no operator symbols.
* min and max on floats are NaN-suppressing by default: min(x, NaN) = x. NaN-propagating variants are available as
  methods min_propagating and max_propagating on Float for users who need strict IEEE 754 semantics.
* Negative integer exponent on integer base ((2).pow(-1)) traps. Programmer error when the result type is integer;
  explicit cast to float required for fractional results.
* Logarithm naming avoids the natural-vs-base-10 ambiguity entirely: ln(x) for natural log, log2(x) for base-2, log10(x)
  for base-10, log(base, x) for arbitrary base. No bare log(x) exists in the language.
* floor/ceil/round/trunc are defined only on floats. Integer ceiling division, floor division, etc. are standard-library
  concerns (e.g., a div_ceil method on Integer if the stdlib chooses to provide it).
  Special values and constants live as associated values on the concrete numeric types, not on traits (because they are
  exact values whose representation depends on the concrete type):
* Float constants: f32::PI, f64::PI, f32::E, f64::E, f64::TAU, f64::LN_2, f64::LN_10, f64::INFINITY, f64::NEG_INFINITY,
  f64::NAN, etc.
* Integer extremes: i32::MIN, i32::MAX, u8::MAX, i64::MIN, etc.
* NaN/infinity inspection methods on Float: x.is_nan(), x.is_infinite(), x.is_finite(), x.is_normal().
  Implementation: trait abstraction at the language level; compiler optimizes aggressively. The standard library's
  implementation of Sqrt::sqrt for f32 may be a one-line call to a compiler intrinsic that emits the platform's SQRT
  instruction directly. Users see the trait method; the emitted code is direct. This is an implementation concern; the
  language model treats these as ordinary trait methods.
  Rejected: bare free-function form requiring no trait import (would introduce an implicit name-resolution rule beyond
  the existing trait-method machinery — better to require the import, making the dependency visible); free-function form
  as a separate definition from the method form (duplicate definitions kept manually in sync — error-prone, redundant);
  compiler intrinsics outside the trait system (would create two-tier numeric operations where built-in math is special
  and user-defined operations are second-class); single combined Pow trait (the integer-vs-float semantics differ enough
  that fine-grained separation pays off — negative exponents trap on IntPow but produce fractional results on FloatPow);
  bare log(x) (ambiguous across language traditions, source of bugs); NaN-propagating min/max as the default (most user
  code wants NaN-suppressing behavior; the propagating variant is the specialist case). Topic 14: Conversion traits.
  User-controlled conversions beyond the built-in widening rules use a pair of trait pairs: From/Into for infallible
  conversions and TryFrom/TryInto for fallible conversions.
  The four traits:
  trait From[T]:
  fn from(value: T) -> Self

trait Into[T]:
fn into(self) -> T

trait TryFrom[T]:
type Error
fn try_from(value: T) -> Result[Self, Error]

trait TryInto[T]:
type Error
fn try_into(self) -> Result[T, Error]
Users implement From and TryFrom. The reverse direction is auto-derived: Into[U] for T is automatically provided
whenever From[T] for U exists, and TryInto[U] for T is automatically provided whenever TryFrom[T] for U exists (with the
same associated Error type). The auto-derivation is language-built-in and not user-overridable, preventing coherence
violations between disagreeing manual impls.
The fallibility split is semantic, not stylistic. From/Into is for conversions that cannot fail — widening,
semantic-preserving transformations, identity. TryFrom/TryInto is for conversions that can fail — narrowing, parsing,
range checks, validation. The trait the user implements signals the fallibility to every caller. Fallible conversions
return Result[T, Error] where Error is an associated type on the Try trait, letting each conversion specify what kind of
failure it produces (range error, parse error, validation error, etc.). Errors are typed, consistent with Topic 3's
nominal trait model.
Identity conversion (From[T] for T) is auto-implemented for every type. Useful for generic code expressing "this type or
anything convertible to it."
Built-in numeric conversions populate these traits according to Topic 6's lossless rules:

* From impls for all lossless conversions: integer-to-wider-same-signedness, unsigned-to-wider-signed,
  float-to-wider-float, integer-to-float for exact-representable cases (i8/u8/i16/u16 to f32; i32/u32 to f64), and the
  Topic 6 pragmatic exception From[i64] for f64 / From[u64] for f64.
* TryFrom impls for narrowing, signed/unsigned crossing, and lossy integer-to-float conversions. Each carries an
  appropriate Error type (typically a numeric range error).
  Relationship to the as operator from Topic 7d and Topic 10:
* For lossless conversions, x as U and x.into::[U]() produce the same result. Both are valid; users pick based on style.
* For lossy conversions, as traps at runtime per Topic 10; try_into returns Result[T, Error] for explicit handling. They
  are different operations with different failure semantics. Users pick based on intent — panic-on-failure (as) or
  recoverable error (try_into).
* The wrapping and saturating cast variants from Topic 10 (wrapping_as, saturating_as, etc.) are method forms on the
  integer types, not part of the conversion-trait machinery.
  Implicit conversion surface is unchanged from Topic 6: only built-in lossless widenings are implicit. User-defined
  From impls do not produce implicit conversions; users write .into() or From::from() explicitly at the call site. This
  keeps the rule "what gets converted implicitly" strictly bounded by Topic 6 and not extensible by user impls.
  Invocation forms follow Topic 13's pattern (method, pipe-forward, free-function via trait path):
  let x: f64 = (5_i32).into::[f64]()      // method form
  let x: f64 = 5_i32 >> Into::into // pipe-forward
  let x: f64 = From::from(5_i32)          // free-function via trait path
  let r: Result[i32, _] = big.try_into::[i32]()
  Rejected: bidirectional pair with both From and Into user-implementable (coherence becomes hard; disagreeing impls
  possible); single bidirectional Convert trait (forces every conversion to be bidirectional, which isn't always
  semantically true; doesn't compose with fallibility); merging fallible and infallible into one trait (puts
  error-handling burden on every conversion site, even where the conversion can't fail; obscures the type-system
  signal); making From impls produce implicit conversions (would expand the implicit-conversion surface beyond Topic 6's
  tight rules, reintroducing the C-family hazard of action at a distance through user-defined conversions).

Topic 15: Trait coherence and orphan rules.
A trait implementation impl Trait for Type is permitted in module M if and only if at least one of:

* Trait is defined in M, or
* Type is defined in M.
  This is the strict orphan rule. There is no exception for "private" or "non-exported" impls. The rule is structural:
  an impl whose trait and type are both foreign to the current module is rejected at the impl declaration, not at use
  sites.
  The rule guarantees coherence across the entire module graph: two independent modules cannot write conflicting impls
  for the same (type, trait) pair, because at least one of them would violate the orphan rule. This composes cleanly
  with separate compilation, monomorphization, and predictable error messages. The cost — sometimes needing a newtype
  wrapper to implement a foreign trait for a foreign type — is bounded and mechanical.
  Generic-parameter coverage applies to impls involving type parameters. impl ForeignTrait[LocalType] for ForeignType is
  permitted because LocalType is local and "covers" the impl. impl ForeignTrait[T] for ForeignType with T unconstrained
  is rejected: the impl is orphan because no local concrete type appears in any position. For impl Trait[T] for
  ForeignType, at least one type parameter (in Trait's parameter list or Type's parameter list) must be a local concrete
  type.
  The newtype pattern is the canonical workaround when a user wants to implement a foreign trait on a foreign type. The
  grammar's distinction between alias type (no new identity, §3.5.1) and type (new identity) makes the difference
  explicit: aliases are transparent to the orphan rule (they substitute literally); newtypes count as local types and
  satisfy the rule. Example using satisfies-first convention:
  type MyVec:
  satisfies SomeForeignTrait
  inner: Vec[i32]
  The newtype MyVec is local to its defining module; impl SomeForeignTrait for MyVec (or the satisfies declaration
  shown) is orphan-rule-compliant.
  Language-privileged impls are not subject to the orphan rule:
* Auto-implementations of built-in numeric traits for built-in numeric types (Topic 3). These are structurally provided
  by the language, not written as impls in any user module. User code cannot redefine them, so no conflict is possible.
* Auto-derivations from From to Into and TryFrom to TryInto (Topic 14). These are structural derivations the language
  guarantees exist whenever the source impl exists. They are not "impls" in the user-writable sense.
* Identity conversion From[T] for T for every type (Topic 14). Universally provided by the language.
  Umbrella traits from Topic 9 (Numeric, Integer, Float, etc.) are not directly implemented by users. A user
  implementing Integer for a custom type implements the fine-grained component traits (Add, Sub, Mul, Rem, IntDiv,
  bitwise, shifts, etc.); umbrella satisfaction follows structurally from satisfying all components. There is no
  separate "impl Integer for MyType" that could conflict with the component impls. The umbrella is a constraint
  shorthand, not an implementable trait.
  Default method bodies in trait declarations (per the grammar's §3.7 TraitFnDecl with optional body) live in the
  trait's defining module. Concrete impls in user modules can override them. The orphan rule applies to concrete impls
  in user modules, not to default bodies in trait declarations.
  Rejected: looser orphan rule permitting "private" orphan impls (the privacy boundary is hard to enforce cleanly,
  creates surprising leak cases, and confuses the user's mental model of when impls are visible); modular coherence with
  whole-program checking (incompatible with separate compilation from Topic 8, doesn't compose with dynamic loading,
  requires global analysis); no orphan rule at all (allows direct conflicts and incoherence, making the trait system
  unsound across module boundaries). Topic 16: Bool and char in the numeric system.
  bool and char are primitive types but are not numeric. Neither implements Numeric, Integer, Float, or any arithmetic
  trait from Topic 9. Arithmetic operations on bool or char are type errors.
  Both implement Eq and Ord:
* bool orders as false < true by convention. Used in is/is not comparisons, ordered collections, pattern matching, and
  any generic code constrained on Ord or Eq.
* char orders by Unicode scalar value. Total order, NaN-free. Same usage surface as bool.
  Logical operations on bool (and, or, not) are language-level keywords per the grammar's §3.15. They are not trait
  methods and do not participate in the numeric trait system.
  Conversion between bool/char and numeric types goes through Topic 14's conversion traits:
* From[bool] for i32 (and similar for other integer types) is infallible: true → 1, false → 0. No reverse direction is
  auto-provided; integer-to-bool requires the user to write explicit logic (e.g., n is not 0).
* From[char] for u32, From[char] for i32, From[char] for i64, etc., are infallible — every Unicode scalar value fits in
  u32 or wider.
* TryFrom[u32] for char is fallible — not every u32 is a valid Unicode scalar (surrogates 0xD800–0xDFFF and values above
  0x10FFFF fail). The associated Error type identifies invalid scalar values.
* TryFrom[char] for u8 is fallible — only ASCII chars (0x00–0x7F) fit. The associated Error type identifies non-ASCII
  chars.
  Character iteration ("next char") is therefore explicit, exposing the subtlety that not every increment produces a
  valid char:
  let next: char = ((c as u32) + 1).try_into::[char]()?
  The standard library may provide ergonomic helpers (c.next() -> Option[char]) for common iteration cases without
  making char itself numeric.
  Byte literals (grammar §2.5.4) produce values of type u8. They are u8 in every type-system sense — fully numeric,
  participate in all integer traits and arithmetic. b'a' + 1 is valid u8 arithmetic. No special-casing; byte literals
  are syntactic sugar for u8 values derived from ASCII character notation.
  The "Numeric" set is bounded and opt-in. Types implementing Numeric are:
* All built-in integer types (i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, usize, isize).
* All built-in float types (f32, f64).
* User-defined types that explicitly implement the numeric trait components per Topic 3 and Topic 9.
  bool, char, byte arrays, time durations, money types, and any other "has a number inside" type are not numeric unless
  the user explicitly opts in by implementing the relevant traits. The default is "not numeric"; opt-in via trait impls
  per Topic 3. This keeps generic numeric code (fn average[T: Numeric](...)) honest about the operations it requires and
  prevents semantically meaningless arithmetic from typechecking.
  Rejected: bool as numeric (the C-family footgun — conflates logical and numeric reasoning, allows bool * int and
  similar nonsense, obscures intent at every call site); char as numeric with full arithmetic (char + char is
  semantically meaningless; conflates code-point identity with integer offset); char with mixed semantics where char +
  int is allowed but char + char isn't (introduces special-case operator-overload rules into the type system; the
  explicit conversion path is more honest); implicit conversion between bool/char and integers (would reintroduce the
  C-family hazard of action at a distance; explicit From/Into puts the conversion at the call site). Topic 17: Explicit
  generic syntax details.
  Generic parameters are uniformly available on all parameterized declarations: functions, types, traits, enums, nodes,
  connections. The grammar's GenericParams production applies consistently across all Decl forms that introduce a name.
  Bound syntax is offered in two equivalent forms, both first-class. Users pick based on readability:
* Inline bound on the parameter: fn sort[T: Ord & Eq](xs: T[N]) -> T[N].
* Where-clause suffix: fn sort[T](xs: T[N]) -> T[N] where T: Ord & Eq.
  For non-trivial bound expressions — long trait paths, bounds on associated types, many parameters — the where-clause
  form is generally more readable. For simple single-parameter bounds, the inline form is more concise. Neither form is
  privileged.
  Bounds on associated types use the where-clause form only, with . for associated-type access:
  fn sum[I: Iterable](it: I) -> I.Item where I.Item: Numeric:
  ...
  The inline form does not nest; reaching into associated types from a parameter bound requires the where-clause. This
  keeps the inline bound surface flat and the syntax uniform with the rest of the language's member-access notation.
  Default type parameters are supported per the grammar's §3.11.1. A parameter may declare a default via = TypeExpr or =
  Expr (for value generics). Defaults can reference earlier parameters in the same parameter list (forward references
  only — no cycles). Defaults are filled in at instantiation when the parameter is omitted; trailing-only omission is
  permitted (a parameter cannot be omitted while a later parameter is specified).
  trait Add[Rhs = Self]:
  type Output
  fn add(self, other: Rhs) -> Output

type Buffer[T, N: usize = 1024]:
data: T[N]
Const generics (value generics) are supported over types with decidable, cheap structural equality. The supported set:

* All built-in integer types: i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, usize, isize.
* bool.
* char.
* Payload-less enums (the inline-form enum Foo = A | B | C from grammar §3.6).
  Const-generic parameters of these types are distinguished structurally: Array[i32, 5] and Array[i32, 6] are distinct
  types; SomeType[Direction::Up] and SomeType[Direction::Down] are distinct types. Compile-time-known expressions of
  these types per Topic 11 are valid const-generic arguments: let arr: i32[fib(10) + 1] is well-formed because fib(10) +
  1 evaluates to a compile-time-known integer.
  Payload-carrying enums, floats, strings, records, and other types with non-trivial or expensive structural equality
  are not supported as const-generic parameter types in this version. The restriction is conservative; the language is
  not painted into a corner by it. Future extension to broader types is possible by extending the structural-equality
  machinery without rewriting the rest of the type system.
  Variance markers are not provided and there is no subtyping relationship between distinct generic instantiations.
  Container[Cat] and Container[Animal] are unrelated types regardless of any relationship between Cat and Animal. The
  grammar's §6 reserves variance markers for possible future extension; the current language does not expose them.
  Higher-kinded type parameters are not part of this version. The grammar's §6 reserves the syntax (F[_]); the
  implementation reserves the type-system slot. The compiler's internal representation of generic parameters models
  kinds explicitly — every parameter carries a kind, currently restricted to the single concrete kind * (the kind of
  ordinary types). This internal modeling enables future extension to higher-kinded types without rewriting the
  type-checker, while keeping the user-facing surface free of kind annotations or higher-kinded syntax in v1.
  Implicit (inferred-type) generic functions and explicit generic functions are equivalent. Writing fn lerp(a, b, c):
  a + (b - a) * c desugars to fn lerp[T0, T1, T2](a: T0, b: T1, c: T2): a + (b - a) * c with fresh, unique type
  parameters per omitted parameter type and inferred constraints attached per Topic 2. Mixed forms are permitted: a
  function may have some parameter types explicit and others omitted. The expressive power is identical regardless of
  which form the user writes; the choice is stylistic.
  Trait conjunction uses & at all positions, uniformly: declaration-site parameter bounds (T: A & B), where-clause
  bounds (where T: A & B), use-site type intersections (fn pick[T: A & B](...), to: A & B on connections per grammar
  §3.9, type X = A & B for record intersection per grammar §3.16). One operator, one meaning across all positions,
  consistent with the grammar's §3.11.1 and §3.16.
  Type argument inference at call sites is the default. When the compiler can determine generic type arguments from the
  call's argument types, no explicit annotation is needed. Explicit instantiation is available as a fallback for
  ambiguous cases or when the user wants to pin the types deliberately. The explicit form uses the turbofish
  syntax ::[Type]:
  let r = lerp(0.0_f64, 1.0_f64, 0.5_f64)        // T inferred from arguments
  let r = lerp::[f64](0.0, 1.0, 0.5)             // T explicit via turbofish
  The :: prefix on the type-argument list is grammatically necessary to disambiguate from indexing in expression
  position. Without it, foo[T](args) would be ambiguous between "index foo with T, then call" and "call foo with type
  argument T." The :: forces the parser into path-navigation mode, where [T] is unambiguously a generic type-argument
  list. This is consistent with the language's existing use of :: for path navigation (f32::PI, Result::Ok); generic
  instantiation is conceptually a path operation that selects the monomorphized variant.
  Rejected: nested inline bounds on associated types (Rust-style [I: Iterable<Item: Numeric>] — adds nested syntax to
  the parameter list, less readable than the where-clause form for non-trivial cases); variance markers in v1 (premature
  for a language without nominal subtyping; the grammar reserves the syntax for future extension); higher-kinded types
  in v1 (significant complexity increase, not needed for any locked-in topic, deferred with implementation-level
  future-readiness); making implicit and explicit generic forms semantically distinct (forces users to learn two
  mechanisms with subtle differences); restricting const generics to usize only (artificially narrow; bool, char,
  integers, payload-less enums all satisfy the structural-equality requirement and are useful); inline [Type] for
  turbofish without :: (creates parser ambiguity with indexing). Locking with those corrections.
  Topic 18: Standard numeric type set.
  The built-in numeric primitive type set is fixed at fourteen types:
  Signed integers: i8, i16, i32, i64, i128, isize.
  Unsigned integers: u8, u16, u32, u64, u128, usize.
  Floating-point: f32, f64.
  i128 and u128 are first-class types. The performance overhead on platforms without native 128-bit operations is
  bounded and paid only when used; the alternatives (standard-library big-integer types, manual u64 pairs) are
  dramatically worse ergonomically for the legitimate use cases (UUIDs, cryptography, high-precision fixed-point,
  financial domains needing more than 64-bit range).
  Both isize and usize are first-class platform-sized integer types. They are distinct types serving distinct roles:
  isize is the array length and index type per Topic 12, and the natural choice for general signed platform-integer
  work; usize exists for FFI compatibility with C-family APIs taking size_t, byte-count contexts where the non-negative
  invariant is load-bearing, and low-level memory layout work. Most code uses isize; usize appears in low-level corners.
  f32 and f64 are the core float types. Both are universally supported on every relevant platform with native hardware
  operations and cover the overwhelming majority of float-using code. f16 (half-precision) and f128 (
  quadruple-precision) are deferred to future versions — f16's hardware story remains uneven and many of Topic 13's
  transcendental operations would require f32 fallback; f128 is highly specialized and adequately served by
  standard-library arbitrary-precision types when needed.
  Convenience aliases are provided out-of-the-box using the language's existing alias type mechanism per grammar §3.5.1.
  Aliases are true aliases — transparent substitution, shared identity, fully interchangeable with the underlying type
  at every use site:
  alias type byte = u8
  alias type short = i16
  alias type int = i32
  alias type long = i64
  alias type double = f64
  A value of type int is a value of type i32 — same machine representation, same trait impls, no conversion needed at
  any boundary. The aliases are conveniences for users who prefer C-traditional names; the canonical names (u8, i16,
  i32, i64, f64) remain the language's primary identifiers and appear unaltered throughout the standard library and
  language documentation.
  The aliases come from the standard library, not the language core, using the same alias type mechanism available to
  user code. Users can define their own aliases anywhere using the same syntax. The built-in aliases are a stdlib
  convenience; nothing about them is privileged.
  No alias is provided for f32 — the C-traditional float name would conflict with the lowercase float placeholder
  keyword from Topic 7e (and the user's revised naming-conventions decision), and float = f64 would actively mislead
  users coming from C where float is single-precision. double is the clean answer for f64; users wanting f32 write f32.
  No alias is provided for i128, u128, isize, usize, i8, u16, u32, or u64 — these types have no widely-shared
  traditional short name across language families, and the explicit-width names are clearer than any alias would be.
  Trait-level defaults from Topic 4 are confirmed against the final type set: numeric → i32, integer → i32, signed →
  i32, unsigned → u32, float → f64. These are the workhorse types in their respective categories and match modern
  language convention.
  Decimal arithmetic, fixed-point arithmetic, arbitrary-precision integers, and other specialized numeric domains are
  standard-library concerns. The numeric trait system from Topics 3, 9, and 14 supports user-defined types implementing
  Numeric and friends cleanly, so stdlib types (Decimal, BigInteger, Fixed[Scale], etc.) integrate via the same
  nominal-impl mechanism as any user-defined type. No special-casing in the language core.
  Non-numeric primitive types per Topic 16 (bool, char) are part of the language but are not numeric in any sense — they
  do not implement Numeric or any arithmetic trait. They appear in this summary only for completeness.
  The numeric type system is closed. Adding new built-in numeric types requires language-level changes; adding new
  numeric-like types via the trait system requires only library-level changes.
  Rejected: omitting i128/u128 (real use cases, software emulation overhead is bounded); making i128/u128 stdlib-only (
  worse ergonomics, fights against the trait system's design); omitting usize and using only isize (forces awkward casts
  at FFI boundaries and in low-level work where the non-negative invariant is load-bearing); f16 or f128 in v1 (hardware
  support and trait integration not yet justified by the use cases); float alias for f64 (conflicts with the placeholder
  keyword and misleads C-family users); newtype-style aliases (the language already distinguishes alias type for true
  aliases from type for newtypes — aliases here mean true aliases); built-in decimal as a language-core type (
  representation choice, hardware story, and trait-hierarchy implications are heavyweight for a feature better served by
  a stdlib type integrating via the existing trait system). Topic 19: Records and uniform function call syntax.
  Records are nominal types declared via type Name: ... with field bodies. They are pure data — declarations contain
  fields and satisfies clauses, nothing else. No fn declarations appear in record bodies. The self keyword does not
  exist. The extend mechanism does not exist.
  type Person:
  satisfies Display
  first_name: string
  last_name: string
  age: i32
  Behavior on records is provided by ordinary free functions. A function whose first parameter is of type T can be
  invoked with method-call syntax x.f(args), with pipe-forward syntax x >> f: args, or with conventional call syntax f(
  x, args). All three forms are equivalent — they desugar to the same function call. There is one concept (function)
  with three call-site syntaxes:
  fn full_name(person: Person) -> string:
  "{person.first_name} {person.last_name}"

let name = p.full_name()        // method-call form
let name = p >> full_name // pipe-forward form
let name = full_name(p)          // conventional form
Method-call dispatch resolves via ambient name resolution: x.f(args) looks up f in the current module's scope (imports,
local definitions). Standard scoping rules apply. If f is ambiguous (defined in multiple imported modules with matching
first-parameter type), the compiler emits a name-conflict error at the call site, same as for any ambient name conflict.
The user resolves the conflict by qualifying the call (module::f(x, args)) or by adjusting imports.
This applies uniformly to records, nodes, and connections. None of these declaration forms permit fn declarations in
their bodies. The grammar revisions:

* TypeBodyItem (§3.5): permits SatisfiesClause and TypeFieldDecl. Drops FnDecl.
* NodeDeclItem (§3.8): permits SatisfiesClause, PartsClause, InClause, OutClause, AttrDecl, DerivedAttrDecl. Drops
  FnDecl.
* ConnectionBodyItem (§3.9): permits FromClause, ToClause, AttrDecl, DerivedAttrDecl. Drops FnDecl.
* SelfParam (§3.11): removed entirely.
* ExtendItem (§3.12): removed entirely.
  derived and attr declarations remain in node and connection bodies. These declare reactive interfaces of the type, not
  methods on instances — they describe the type's structure (its reactive surface, its attributes), not behaviors
  invoked on values. The asymmetry between "no fn in record/node/connection bodies" and "derived/attr permitted in
  node/connection bodies" is principled: records/nodes/connections declare what the type is; functions declare what can
  be done with values of the type.
  Trait bodies (§3.7) continue to permit fn declarations. Traits declare abstract operations as signatures (with
  optional default bodies). This is distinct from putting methods on values — trait bodies describe an interface, not an
  instance's behavior. The grammar's TraitFnDecl is unchanged.
  Trait conformance is verified by matching free functions against trait method signatures. When a type T declares
  satisfies SomeTrait, the compiler verifies that for every required method fn op(self, args...) -> R in SomeTrait,
  there exists a free function fn op(value: T, args...) -> R in scope, subject to the orphan rule from Topic 15 (the
  function must be defined in a module that owns either the trait or the type). The satisfies clause is the explicit
  nominal declaration; the function-existence check is the structural verification of that declaration.
  Trait method signatures use Self as the receiver type, since self no longer exists:
  trait Display:
  fn display(value: Self) -> string

trait Add[Rhs = Self]:
type Output
fn add(left: Self, right: Rhs) -> Output
The first parameter name (value, left, etc.) is the trait author's choice. Call sites use whichever form reads best:
p.display(), p >> display, Display::display(p), display(p) (when imported).
Visibility and privacy apply to free functions the same as any function. A private function fn helper(p: Person) defined
in module M cannot be called as p.helper() from outside M. No special privacy rules for "methods."
Discoverability of operations on a type is tooling's responsibility. The compiler and language server can answer "what
functions take Person as first parameter and are reachable from this scope" — a structural query equivalent to "what
methods does Person have" in OOP languages. Source code does not group these functions syntactically; convention may
group them via doc-comment sections or module organization, but the language does not require or enforce grouping.
Rejected: methods directly in record bodies (Rust/Swift style — conflates data declaration with behavior, contrary to
the principle that records are pure data); extend blocks for behavior addition (introduces a separate mechanism for what
can be expressed as free functions; adds a concept the language doesn't need); self keyword for receiver parameters (
saves three characters per signature but creates a special name with non-standard scoping rules; the explicit parameter
name is uniformly clearer); type-qualified or receiver-defining-module-only dispatch (more restrictive than necessary,
less flexible for cross-module helper functions, doesn't match how the rest of name resolution works in the language);
making trait implementations a single syntactic unit (would require either impl blocks or extend blocks, both of which
the design eliminates — distributed free functions are the consequence of choosing uniform function call syntax).

Locked.
Topic 19a: Nodes, connections, and self for reactive context.
Nodes and connections follow the same uniform-function-call rule as records: their declaration bodies are pure structure
declarations and contain no fn definitions. Behavior on node and connection values is provided by free functions whose
first parameter is the node or connection type, callable via method-call, pipe-forward, or conventional syntax per Topic

19.

The grammar revisions from Topic 19 apply: NodeDeclItem permits SatisfiesClause, PartsClause, InClause, OutClause,
AttrDecl, DerivedAttrDecl but not FnDecl. ConnectionBodyItem permits FromClause, ToClause, AttrDecl, DerivedAttrDecl but
not FnDecl. The extend mechanism does not exist for nodes or connections.
The self keyword is retained, but only for use inside node and connection bodies. Specifically, self is the implicit
reference to the encompassing node or connection instance, available inside derived and attr expressions, and inside any
expression evaluated within the node or connection's body scope. This is necessary because reactive declarations (
derived, attr with computed defaults) need to reference other attributes of the same instance:
node Driver:
satisfies Drivable, Insurable
attr expertise_level: i8
attr risk_tolerance: f32 = 0.5
derived skill_factor: f32 = self.expertise_level as f32 / 10.0
derived aggressive: bool = self.risk_tolerance > 0.7
self.expertise_level inside skill_factor's expression refers to the same instance's expertise_level attribute. Without
self, the reactive declarations would have no syntactic way to refer to the instance whose attributes they project from.
self in this context is not a function parameter (no SelfParam from §3.11; that production is removed per Topic 19). It
is a scope-local keyword visible inside node and connection declaration bodies, resolving to the instance currently
being declared or constructed. Its semantics are reactive-scope-bound: references through self participate in the
reactive dependency graph (Topic 11), so a derived reading self.x becomes reactive on changes to x.
self is not available in record bodies, because records contain no expressions that need it (fields are declared, not
computed in terms of other fields). Records that need computed fields can use newtype wrappers or stdlib helpers, but
they do not have a reactive layer.
self is not available in trait bodies. Trait method signatures use Self (the capitalized type-level identifier) for the
receiver type, as locked in Topic 19. The lowercase self keyword is exclusively the reactive-instance reference inside
node and connection bodies.
self is not available in free functions, even when those functions take a node or connection as their first parameter.
Such functions reference the instance via their parameter name, the same as for records:
fn aggressive(driver: Driver) -> bool:
driver.risk_tolerance > 0.7
The asymmetry — self inside the node body, parameter name in free functions operating on nodes — is principled: self
belongs to the reactive-context-of-declaration; free functions have no reactive context, only an ordinary value
parameter.
Grammar revision: self becomes a context-restricted keyword. Reserved in §2.4.1 (already is). Valid as an expression
only inside NodeBody and ConnectionBody per §3.8 and §3.9. Outside those contexts, using self is a parse-or-semantic
error with a message indicating the restriction.
Rejected: self available everywhere (overly permissive; conflicts with Topic 19's uniform-function-call design for
records); self available in record bodies (records have no reactive context; the keyword would have no meaning); self in
free functions taking node/connection parameters (creates two ways to refer to the same value within different contexts,
confusing); eliminating self and forcing reactive declarations to use explicit parameter-like syntax (verbose and
unnatural for reactive expressions inside an instance's own declaration). When trait defines methods, compilers verifies
that implementing type "satisfies" the contract by checking locally to the type-declaring module. The orphan rule
already enforces that the implementations live there (or in the trait-defining module); we just additionally require
that the implementations be present at the point of declaration. This means: when you write satisfies Display in Event's
declaration, the compiler immediately checks that fn display(e: Event) -> string exists in the same module (or in the
module defining Display, per orphan rule). If it doesn't, that's a compile error at the type declaration, not at some
far-away call site. For Event: satisfies Display, the compiler looks for a function with:

* Name display (matches the trait method name exactly).
* First parameter type Event (the implementing type, substituted for Self).
* Remaining parameters and return type matching the trait signature (after Self substitution).
* Visibility: at least as visible as the trait's method requires — if Display::display is pub, the implementation must
  be pub.
  Locking both in separately.

Topic 19b: Trait implementations via fulfill blocks.
Trait conformance is implemented through explicit fulfill Trait for Type blocks, not through free functions matching
trait method signatures by name. A fulfill block is a syntactic unit that declares "this type delivers on this trait's
contract" and contains the function definitions required by the trait.
fulfill Display for Event:
fn display(e: Event) -> string:
match e:
KeyPress(key): "key: {key}"
Click(at): "click at ({at.x}, {at.y})"
Quit: "quit"
The satisfies clause in the type body and the fulfill block in some module are paired. The type's body declares the
contract; the fulfill block delivers the implementation. Both are required:

* satisfies Trait in a type body without a matching fulfill Trait for Type block anywhere in the orphan-rule-permitted
  modules is a compile error — the promise is unfulfilled.
* fulfill Trait for Type without a corresponding satisfies Trait in Type's body is a compile error — the implementation
  has no declared contract.
  This pairing makes the contract visible at the type's declaration site (a reader sees what the type promises without
  leaving the type's file) while permitting the implementation to live elsewhere (subject to the orphan rule from Topic
  15).
  The orphan rule from Topic 15 applies to fulfill blocks: a fulfill Trait for Type block is permitted in module M if
  and only if Trait is defined in M or Type is defined in M.
  Functions inside fulfill blocks have signatures matching the trait's declared method signatures, with Self resolved to
  the implementing type. The receiver parameter is named explicitly (no self keyword per Topic 19); the parameter type
  is either the implementing type or Self:
  fulfill Add for i32:
  type Output = i32
  fn add(left: i32, right: i32) -> i32:
  left + right
  Or equivalently using Self:
  fulfill Add for i32:
  type Output = i32
  fn add(left: Self, right: Self) -> Self:
  left + right
  Default method bodies declared in the trait (per grammar §3.7) are inherited unless overridden in the fulfill block. A
  fulfill block must provide implementations for all abstract methods (those without defaults) and may override methods
  that have defaults.
  Functions inside fulfill blocks are syntactic groupings, not a separate namespace. They are ordinary free functions
  for resolution purposes: a function fn display(e: Event) -> string inside fulfill Display for Event occupies the same
  name slot in the module as a top-level fn display(e: Event) -> string would. The fulfill keyword tells the compiler "
  these functions exist to satisfy this trait's contract" but does not isolate them from ordinary name resolution.
  Consequently, defining both fulfill Display for Event (with a display function inside) and a top-level fn display(e:
  Event) in the same module is a name conflict — the same name cannot be defined twice in one module regardless of
  whether one of the definitions is inside a fulfill block.
  Trait method dispatch at call sites uses ordinary name resolution per Topic 19's uniform call syntax. event.display(),
  event >> display, display(event), and Display::display(event) all resolve display through the normal scoping rules,
  find the function defined inside the fulfill block (which is in the module's namespace by virtue of being a syntactic
  grouping, not a separate scope), and call it. The trait-qualified form Display::display(event) is available for cases
  where multiple traits define methods with the same name and disambiguation is needed.
  Grammar addition:
  FulfillItem := 'fulfill' TypeExpr 'for' TypeExpr FulfillBody
  FulfillBody := NEWLINE INDENT FulfillBodyItem+ DEDENT
  FulfillBodyItem := Annotation* DocComment? (FnDecl | AssocTypeBinding)
  AssocTypeBinding := 'type' Ident '=' TypeExpr NEWLINE
  fulfill is added as a reserved keyword. AssocTypeBinding provides values for the trait's associated types (e.g., type
  Output = i32 in the example above).
  Rejected: implicit trait conformance via free function name+signature matching (loses the explicit contract
  declaration; readers can't see what a function is for from its definition site; intent is invisible); making fulfill
  blocks a separate namespace from free functions (creates the case where a free function and a trait-implementation
  function with the same name and same first-parameter type can coexist in one module — confusing dispatch); making
  satisfies optional when fulfill exists (loses the at-a-glance contract visibility on the type's declaration site,
  which was the original motivation for satisfies).

Topic 20: Function identity and name resolution
Free functions are uniquely identified by their fully-qualified path: module path plus local name. Within a single
module, every function name is unique — defining two functions with the same name in the same module is a compile error,
regardless of their parameter types. The language does not support ad-hoc overloading.
Different modules can each define functions with the same local name. car_module::get_color(car: Car) and boat_module::
get_color(boat: Boat) coexist because their fully-qualified paths differ.
Function resolution at call sites follows ordinary name resolution rules:

* A bare name f resolves to a function in the current module's scope. The scope includes locally defined functions and
  functions imported via use.
* A path-qualified name module::f resolves through the module path directly, no import required.
* Ambiguity (two use imports bringing different functions with the same local name into scope) is resolved at the import
  site, not at the call site. The user either aliases one import (use car_module::get_color as car_get_color) or omits
  the conflicting import and path-qualifies the calls.
  Calling a function from another module requires either path qualification or import:
  // Path-qualified call, no import
  car_module::get_color(car)

// Function-level import, then call by local name (any of three forms)
use car_module::get_color
get_color(car)         // conventional
car.get_color()        // method-call sugar
car >> get_color // pipe-forward
Per the grammar's §3.2, imports take specific names from a module (use path::Name), groups of names (use path::(Name1,
Name2)), all public names from a module via glob (use path::*), or aliased names (use path::Name as Alias). There is no
bare module-as-name import; module names are always traversed via path syntax (car_module::get_color) without a separate
import.
Files in the same folder are auto-visible to one another without use per grammar §3.2 — same-folder modules see each
other's public names directly. Cross-folder visibility requires explicit pub on the items being exported and routes
through optional index files per the semantics document (out of grammar scope).
Method-call resolution (x.f(args)) uses the same name resolution as f(x, args). The compiler resolves f in the current
scope using ordinary rules, then verifies that the function's first parameter type matches x's type (or is reachable via
implicit widening per Topic 6). If f is not in scope or x's type doesn't match, the call fails — the same way a
conventional call would fail. There is no separate "method lookup" mechanism.
Trait method dispatch is the resolution path for functions defined inside fulfill blocks per Topic 19b. Since fulfill
blocks are syntactic groupings rather than separate namespaces, their functions participate in ordinary name resolution
alongside free functions. A trait method declared as Display::display(event) works because Display::display is a path
through the trait's namespace to the function inside the relevant fulfill block. The trait acts as a kind of secondary
namespace for its methods, accessible via the trait's path; the functions themselves are also reachable through ordinary
name resolution if the trait is imported into scope.
Generic functions (Topic 17) are identified by their fully-qualified path, same as non-generic. Monomorphization (Topic

8) produces per-call-site instantiations tagged with the full path, ensuring instantiations across modules don't
   collide.
   Internal compiler representation: each function has a fully-qualified identifier (module path + local name). No name
   mangling for parameter types is required — the path already disambiguates, and there is no ad-hoc overloading to
   mangle.
   Generic monomorphization tags instantiations with the parameter types in the symbol, but this is a code-generation
   detail invisible to the type system.
   Rejected: ad-hoc overloading (Option A — creates complex interaction with generics, monomorphization, and uniform
   call
   syntax; modern systems languages have mostly converged against it); function-name uniqueness across the entire
   project (
   Option B without modules — verbose and unnecessary given the module system already provides namespacing); separate
   lookup rules for method-call vs conventional-call (Option C — breaks the uniform call syntax principle from Topic
   19);
   type-scoped functions belonging to a type's namespace (Option D — resurrects something equivalent to methods,
   contrary
   to Topic 19's principle that records are pure data); bare module-name imports (use car_module alone) without
   specifying
   items (not supported by the grammar's use forms; the path-qualified call form covers the same use case without an
   import). Topic 21: Traits and enums.
   Enums participate in the trait system identically to records and other types. There is no enum-specific trait
   machinery — the satisfies clause from the type body and the fulfill block from Topic 19b operate on enums exactly as
   they do on any other nominal type. The natural implementation strategy for enum trait impls uses match inside the
   fulfill block's functions:
   type Event:
   satisfies Display
   KeyPress(key: string)
   Click(at: Vec3)
   Quit

fulfill Display for Event:
fn display(event: Event) -> string:
match event:
KeyPress(key): "key: {key}"
Click(at): "click at ({at.x}, {at.y})"
Quit: "quit"
Trait implementations are per-enum, not per-variant. A fulfill Display for Event block implements Display for the entire
enum type. Variants cannot independently fulfill traits — fulfill Display for Event::KeyPress is rejected because
variants are not types.
Variants are constructors, not types. Event::KeyPress(key: "a") produces a value of type Event, not a value of a
hypothetical type KeyPress. Pattern matching destructures values of the enum type; it does not switch between sub-types.
Variant access uses path syntax from the enum's namespace (Event::KeyPress, Result::Ok, Option::Some), and unqualified
variant names (KeyPress, Ok, Some) are permitted when the enum is imported and the unqualified name is unambiguous in
scope. This works through the same use mechanism as any other import:
use root::core::Option::(Some, None)
use root::core::Result::(Ok, Err)

let x = Some(42)              // unqualified, resolves via import
let y = Option::Some(42)       // explicit path
let r = Ok("done")             // unqualified
Auto-derivation of trait implementations is supported via the @derive annotation per grammar §3.3. A fixed set of traits
is eligible for automatic derivation: Eq, Ord, Hash, Clone, Display, Debug. Derivation works structurally: @derive(Eq)
generates an implementation that compares variant tags and recursively compares payload fields using each field's own Eq
implementation; @derive(Hash) generates a structural hash combining variant tag and payload hashes; @derive(Display)
generates a sensible default format; @derive(Debug) generates a compiler-defined structural format. Derivation requires
every payload field's type to itself satisfy the trait being derived; if any field type doesn't satisfy the trait,
derivation fails with a clear error pointing at the offending field.
A @derive-generated implementation can be overridden by writing an explicit fulfill block. The compiler treats the
explicit block as authoritative when both exist for the same (Trait, Type) pair; the derivation is suppressed for that
combination.
Generic enums support conditional trait conformance via where clauses on the fulfill block. The implementation is
available only when the type parameters themselves satisfy the required traits. Grammar extension to Topic 19b:
FulfillItem := 'fulfill' TypeExpr 'for' TypeExpr WhereClause? FulfillBody
Example:
fulfill Display for Result[T, E] where T: Display, E: Display:
fn display(result: Result[T, E]) -> string:
match result:
Ok(value): "Ok({value.display()})"
Err(error): "Err({error.display()})"
This permits the standard library's Result[T, E] and Option[T] to implement traits like Display, Eq, Hash conditionally
on their type parameters supporting those traits — Result[i32, string] is Display because both i32 and string are;
Result[ClosureType, string] is not Display because closure types typically aren't.
Trait methods may return enum-typed values. The Self-substitution in fulfill blocks resolves Self to the implementing
type throughout the method signatures, including in generic return types:
trait Parse:
fn parse(input: string) -> Result[Self, ParseError]

fulfill Parse for i32:
fn parse(input: string) -> Result[i32, ParseError]:
// implementation
Self in the trait declaration becomes i32 in the fulfill block's signatures, producing Result[i32, ParseError] as the
concrete return type.
Pattern matching on trait-bound generic values is not possible. A generic function fn process[T: SomeTrait](value: T)
sees value only through the operations declared in SomeTrait. The function body cannot pattern-match on value to access
its variants or fields, because the type T is opaque except through trait methods. This is the standard parametricity
property: generic code abstracts over types via traits, not via structure. Generic code that needs to discriminate cases
must do so through trait methods specifically designed for that purpose, not through ad-hoc structural inspection.
Match expressions inside fulfill block functions remain subject to exhaustiveness checking per the existing enum-match
decisions. Adding a new variant to an enum produces compile errors in every fulfill block whose match becomes
non-exhaustive, surfacing the implementations that need updating. This is the principal mechanism for keeping enum
implementations in sync with the enum's variant set.
Rejected: per-variant trait conformance (variants are not types; conflicts with the simpler model where traits attach to
whole enums; introduces complexity in match semantics and dispatch); variants as standalone types (would require
duplicating type machinery for what are conceptually just constructors of their parent enum; contrary to the grammar's
treatment of variants as path-accessed names within the enum's namespace); implicit trait conformance for enums without
explicit satisfies (loses the at-a-glance contract visibility from Topic 19b); auto-derivation for arbitrary traits (
most traits have implementations too domain-specific for mechanical derivation; the fixed set of derivable traits covers
the common cases without committing the language to a complex derivation mechanism). Topic 22: Newtype semantics.
Newtypes are nominal types with distinct identity from their underlying type, declared in two syntactic forms per
grammar §3.5:
Alias-shaped newtype, for transparent single-value wrapping:
type Meters = f64
type Pitch = i8
type Username = string
Record-style newtype, for structured data with named fields:
type Vec3:
x: f32
y: f32
z: f32

type Address:
street: string
city: string
zip: string
Both forms produce nominal types with distinct identity. The alias-shaped form is the appropriate choice when the
underlying type's representation suffices and no field name carries domain meaning. The record-style form is the
appropriate choice when fields have distinct semantic roles. The forms are not interchangeable — they produce different
construction syntaxes and different field-access patterns — but both follow the same trait, conversion, and visibility
rules below.
Trait inheritance from the underlying type is opt-in via @derive. Each trait to be inherited is explicitly listed in the
annotation. The compiler generates structural fulfill blocks for each derived trait, delegating to the underlying type's
implementation but with the newtype as the operand and result types:
@derive(Add, Sub, Display)
type Meters = f64
This generates implementations equivalent to manually written fulfill Add for Meters, fulfill Sub for Meters, and
fulfill Display for Meters blocks that unwrap, apply the underlying operation, and rewrap. Meters + Meters works and
produces Meters; Meters + Kilograms is a type error even when both wrap f64, because the operations are typed
per-newtype. The user designs the newtype's interface deliberately by choosing which traits to derive; nothing is
inherited implicitly.
@derive works on both newtype forms. For alias-shaped newtypes, derivation delegates to the underlying type's
implementation. For record-style newtypes, derivation generates structural implementations operating over the fields (
e.g., @derive(Eq) compares fields pairwise; @derive(Hash) combines field hashes).
A derived implementation can be overridden by writing an explicit fulfill block for the same (Trait, Type) pair, per
Topic 21's auto-derivation rules.
Conversion between newtype and underlying type is explicit in both directions, but uses different syntactic forms:
Construction uses standard call-syntax per grammar §3.15.4. For alias-shaped newtypes, the construction takes the
underlying value positionally: Meters(1.5). For record-style newtypes, fields are named or use shorthand: Vec3(x: 1.0,
y: 2.0, z: 3.0) or Vec3(x, y, z) (shorthand when local names match field names).
Extraction of the underlying value uses the as operator from Topic 7d: meters_value as f64. The conversion is always
lossless — the underlying value is the newtype's representation — so as here is a representation reinterpretation with
no runtime cost. The compiler treats this as a free operation at the codegen level.
The reverse as-direction is rejected: 1.5_f64 as Meters is not valid. Construction goes through the constructor
exclusively. This keeps the semantic distinction visible at every use site: Meters(1.5) reads as "create a Meters
value"; value as f64 reads as "reinterpret as the underlying type." Mixing these into a single as operator would obscure
the semantic difference.
Underlying-value access from inside the newtype's defining module uses the same as cast as external access. There is no
special syntax or reserved field name for "the underlying value." This keeps the model uniform regardless of where the
code is written. For complex newtypes with many internal operations, the user typically writes fulfill blocks for the
operations rather than repeatedly unwrapping; the verbosity of as is bounded by this practice.
Type visibility and constructor visibility are separately controllable using the public keyword (which replaces the
grammar's pub throughout the language; grammar revision propagates to every visibility position). Default visibility is
module-local; public exports.

* public type Email = string exports the type but does not automatically export the constructor. External code can name
  Email, hold values of type Email, and extract via as, but cannot construct Email values directly.
* An explicit constructor-visibility annotation makes the constructor public: public(constructor) type Email = string,
  or equivalent syntax to be finalized in the semantics document. The exact spelling is deferred; the semantic decision
  is that the two visibilities are independently controllable.
  This enables the smart-constructor pattern: a newtype represents a validated invariant, and the only way to create one
  is through a validated function in the defining module:
  public type Email = string

public fn parse_email(input: string) -> Result[Email, ParseError]:
if is_valid_email_format(input):
Ok(Email(input))    // constructor accessible inside the defining module
else:
Err(ParseError::InvalidEmail)
External code uses parse_email to obtain Email values; direct construction Email(garbage) from outside the module is
rejected because the constructor is not exported.
Extraction visibility follows type visibility: if external code can name the type, it can extract the underlying value
via as. The strict-encapsulation case (where even extraction is forbidden) is not addressed by the newtype mechanism
directly; it requires a different abstraction (e.g., a trait that exposes only specific read methods, with the newtype's
underlying value hidden behind the trait interface).
Newtype trait implementations and methods follow the uniform rules from Topics 19 and 19b. The newtype body permits
satisfies clauses. Trait implementations live in fulfill Trait for NewtypeName blocks. Non-trait operations on newtype
values are ordinary free functions. The @derive mechanism is a convenience for generating common fulfill blocks
structurally; manual fulfill blocks are always available and override derived ones.
Rejected: automatic trait inheritance from the underlying type (defeats the purpose of distinct identity by giving the
newtype every operation regardless of domain meaning); implicit conversion between newtype and underlying type (defeats
the purpose of distinct identity by making the distinction disappear at every assignment boundary); using as for both
directions of conversion (obscures the semantic difference between construction and extraction); single newtype form
requiring all newtypes to be either alias-shaped or record-style (the two patterns serve genuinely different stylistic
purposes — single-value wrapping vs structured data — and the grammar already supports both cleanly); coupling type
visibility and constructor visibility (prevents the smart-constructor pattern, which is a primary use case for
newtypes); extraction visibility separate from type visibility (adds a visibility axis with marginal benefit; the rare
encapsulation case is better served by abstracting through traits than by adding a new visibility category).
Grammar revision: replace pub with public throughout the grammar. Affects §3.4 (signal/derived declarations), §3.5 (
type/field declarations), §3.6 (enum declarations), §3.7 (trait declarations), §3.8 (node declarations and attr
declarations), §3.9 (connection declarations), §3.10 (node instantiation), §3.11 (function declarations), §3.12 (
extend — already removed per Topic 19). The Pub production becomes Public. No semantic change; rename only.

Topic 23: Type intersection.
The & operator means "satisfies all of these constraints simultaneously" across every context in which it appears.
Concrete semantics vary by the kinds of operands and the position of the expression, but the unifying intuition is
uniform.
Three contexts where & appears, each with distinct concrete meaning:
Trait conjunction in generic bounds. Locked in Topics 9 and 17. T: A & B in a generic parameter list or where-clause
constrains T to be a type for which both fulfill A for T and fulfill B for T exist. This is a static constraint resolved
by the compiler at every use site; instantiations are monomorphized per Topic 8 with no runtime dispatch cost. The &
here is a constraint conjunction, not a type expression.
Trait intersection at value position. A variable, parameter, or return type annotated as a trait intersection produces a
trait object that exposes methods from all conjoined traits. Dynamic dispatch through a vtable; runtime cost paid at
each method call. The dyn keyword is required to mark the dynamic-dispatch boundary explicitly:
let x: dyn (Drivable & Insurable) = some_value
fn process(item: dyn (Drivable & Insurable)) -> dyn Renderable
The bare form let x: Drivable & Insurable without dyn is a parse error when both operands are traits. The requirement
makes dynamic-dispatch costs visible at the declaration site rather than implicit in the trait intersection syntax.
Object-safety rules constraining which traits can be used in dyn positions are deferred to the semantics document; the
grammar's §6 already notes this scope split.
Record intersection at type definition. type X = A & B where A and B are record types produces a new nominal record type
with the union of fields from both operands:
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
The resulting type InsuredCar is nominally distinct from both Car and Insured. Values of Car are not implicitly
assignable to InsuredCar (a Car lacks insurance fields); values of InsuredCar are not implicitly assignable to Car (no
implicit projection of fields). Conversion requires explicit construction with all fields or a From impl per Topic 14.
The intersection is a definitional combinator producing a new named type, not a subtyping relationship.
Field merging rules: when A and B declare a field with the same name and identical type, the merged record has a single
field of that name and type. When A and B declare a field with the same name and different types, the intersection is a
compile error with a message identifying the conflict. The user resolves by writing the record explicitly with chosen
field types, or by adjusting the source records.
Trait inheritance for record intersections is opt-in via @derive. The mechanism mirrors newtype derivation from Topic
22a: each trait to be inherited is explicitly listed in the annotation, and the compiler generates fulfill blocks
delegating to the component types' implementations. Ambiguous derivation (both component types have fulfill SomeTrait
blocks that would equally apply) is a compile error; the user must write the implementation manually.
Interaction with alias type per Topic 18:
alias type produces transparent substitution — the alias name and the right-hand-side expression refer to the same
thing, no new identity. The interaction with intersection depends on what the right-hand-side expression evaluates to:

* alias type X = A & B where A and B are traits is valid. The alias names a constraint usable in bound positions: fn
  process[T: X](item: T) is equivalent to fn process[T: A & B](item: T). Useful for naming common bounds for reuse.
* alias type X = dyn (A & B) where A and B are traits is valid. The alias names a dynamic-dispatch trait object type.
  let value: X = ... is equivalent to let value: dyn (A & B) = ....
* alias type X = A & B where A and B are records is rejected at compile time. Record intersection creates a new nominal
  type with combined fields; that creation requires type, not alias type. Without new identity, the intersection has no
  meaning in the language's nominal type system. The compile error directs the user to use type X = A & B instead.
  The asymmetry between trait intersection (aliasable) and record intersection (not aliasable) reflects the deeper
  asymmetry between constraints and types: trait intersection produces a constraint (or a dynamic-dispatch type with
  explicit dyn), which is a single coherent type expression with definite meaning at every use site. Record intersection
  produces fields-combined-into-a-new-type, which only has meaning as a nominal type with identity. Aliases work for the
  former because the right-hand side already has identity (as a constraint or a dyn type); they don't work for the
  latter because the right-hand side requires a type declaration to acquire identity.
  Cross-kind intersection is rejected. Intersection is well-defined only within {trait & trait} and {record & record}.
  Combinations across kinds — Trait & Record, Record & Enum, Trait & Enum, intersection involving tuples, function
  types, or primitive types — are compile errors. The kinds of values these combinations would produce have no coherent
  semantics in the language's type system.
  Variance and intersection do not interact, because there is no variance. Per Topic 17's locked decision, generic
  instantiations are unrelated to each other regardless of any relationship between their type arguments. Each
  instantiation is its own concrete type. Intersection of two distinct generic instantiations (e.g., Container[Cat] &
  Container[Animal]) follows the rules for the resulting kinds: as record intersection, it requires the fields to be
  compatible (typically they are not, since different generic instantiations differ in their field types), so most such
  expressions are compile errors.
  Rejected: structural typing semantics for record intersection (would reintroduce structural typing through a back
  door, contradicting Topic 19's nominal-records decision); bare trait intersection in value-position without dyn (hides
  dynamic-dispatch costs from the call site, contrary to the principle that runtime costs should be visible at the
  source level); automatic trait inheritance for record intersections (parallels the rejected automatic-inheritance
  option for newtypes — gives the new type every operation regardless of domain meaning); cross-kind intersection (no
  coherent semantics for mixing traits with records, records with enums, etc.); alias type over record intersection (no
  transparent identity to substitute; new identity requires type, not alias type). Topic 24: Option, Result, and the ?
  operator.
  Option[T] and Result[T, E] are standard library types built from the generic enum mechanism per Topics 17 and 21. They
  are ordinary enums with no language-level special-casing of their identity. Their definitions live in stdlib:
  enum Option[T]:
  Some(T)
  None

enum Result[T, E]:
Ok(T)
Err(E)
The interactions that look special — the ? operator, error-conversion chains — are mediated through a stdlib trait, not
through compiler knowledge of these specific types. Any user-defined type can participate in ?-propagation by
implementing the same trait.
The ? postfix operator dispatches through a stdlib Try trait. The trait decomposes a value into either "continue with
this success value" or "break with this failure value":
trait Try:
type Success
type Failure
fn branch(value: Self) -> TryBranch[Success, Failure]

enum TryBranch[S, F]:
Continue(S)
Break(F)
Option and Result fulfill Try in stdlib. Try::branch(Some(x)) returns Continue(x); Try::branch(None) returns Break(
None). Try::branch(Ok(x)) returns Continue(x); Try::branch(Err(e)) returns Break(Err(e)). User types can implement Try
to make ? available on their own optional-or-result-like types.
The ? operator desugars to a match on the trait method's result, with the failure branch returning from the enclosing
function, applying From-conversion to bridge failure types:
expr?
==>
match Try::branch(expr):
Continue(value): value
Break(failure): return From::from(failure)
The From::from(failure) automatically converts the failure value into the enclosing function's failure type, using the
From trait from Topic 14. This enables error-type chains: a function returning Result[T, MyError] can use ? on any
Result[U, OtherError] provided fulfill From[OtherError] for MyError exists. The conversion is invisible at the call site
but typed end-to-end; the compiler verifies the From impl exists or rejects with a clear error.
When the failure types are identical (exact match), the From::from call is the trivial identity conversion (fulfill
From[T] for T is auto-implemented per Topic 14). No special-casing for matching types; the same desugaring rule covers
exact-match and conversion cases uniformly.
Cross-type ? is rejected. Using ? on an Option value inside a function returning Result, or on a Result value inside a
function returning Option, is a compile error. The failure-type families are different (Option's None carries no
information; Result's Err carries an error value); silently bridging them would require fabricating an error or
discarding error context. The user converts explicitly via stdlib methods: option.ok_or(SomeError) produces
Result[T, SomeError] from Option[T]; result.ok() produces Option[T] from Result[T, E]. The conversion is visible at the
call site, the failure-handling decision is explicit.
Pattern matching on Option and Result uses standard exhaustive match per Topic 21:
match maybe_value:
Some(x): use(x)
None: handle_absence()

match operation:
Ok(value): proceed(value)
Err(error): handle_error(error)
No special if let or "check-and-unwrap" sugar is provided in v1. The combination of match (for full discrimination)
and ? (for short-circuit propagation) covers the common cases. if let is sugar that may be added later if usage patterns
reveal a sharp need; the current surface is intentionally minimal.
Standard stdlib methods on Option and Result are supported and provided. The full set includes (non-exhaustive list):
For Option[T]:

* unwrap(self) -> T — returns the value or traps if None.
* expect(self, msg: string) -> T — like unwrap with custom trap message.
* unwrap_or(self, default: T) -> T — returns the value or the default.
* unwrap_or_else(self, f: fn() -> T) -> T — returns the value or a computed default.
* map[U](self, f: fn(T) -> U) -> Option[U] — applies a function to the success value.
* and_then[U](self, f: fn(T) -> Option[U]) -> Option[U] — chains optional computations.
* or_else(self, f: fn() -> Option[T]) -> Option[T] — fallback computation.
* ok_or[E](self, err: E) -> Result[T, E] — converts to Result with given error on None.
* is_some(self) -> bool, is_none(self) -> bool — discriminator predicates.
  For Result[T, E]:
* unwrap(self) -> T — returns success or traps on Err.
* expect(self, msg: string) -> T — like unwrap with custom trap message.
* unwrap_or(self, default: T) -> T, unwrap_or_else(self, f: fn(E) -> T) -> T.
* map[U](self, f: fn(T) -> U) -> Result[U, E].
* map_err[F](self, f: fn(E) -> F) -> Result[T, F] — converts error type.
* and_then[U](self, f: fn(T) -> Result[U, E]) -> Result[U, E] — chains fallible computations.
* or_else[F](self, f: fn(E) -> Result[T, F]) -> Result[T, F] — error-recovery chain.
* ok(self) -> Option[T], err(self) -> Option[E] — convert to Option, discarding the other arm.
* is_ok(self) -> bool, is_err(self) -> bool — discriminator predicates.
  All methods are implemented via fulfill blocks in stdlib and callable through Topic 19's uniform function call syntax:
  option.unwrap(), option >> unwrap, unwrap(option), Option::unwrap(option) are equivalent.
  Trapping (Topic 10) and Option/Result propagation are distinct failure mechanisms serving distinct intents:
* Trapping fires for bugs — programmer errors and invariant violations. Overflow, division by zero, out-of-range index,
  unwrap on a None or Err, exhaustiveness failures. Traps halt execution with a diagnostic; they are not catchable as
  values. The right tool when the failure represents a logic error that should be fixed in code, not handled in code.
* Option/Result propagation handles expected conditions — recoverable failures that callers might want to handle.
  Parsing might fail, a file might not exist, a conversion might be out of range, a user input might be invalid. These
  flow through the type system as values; callers decide how to handle them; the ? operator provides ergonomic
  propagation.
  The two are not interchangeable. Once a trap fires, execution halts — it does not become an Err(SomeTrap) value. The
  user picks the mechanism based on the failure's nature when writing the code: traps for "this should never happen if
  the program is correct"; Option/Result for "this might legitimately happen at runtime and the caller might want to
  handle it."
  The arithmetic operator variants from Topic 10 expose this choice directly at the operator level: + traps on
  overflow (the "if this overflows, the program has a bug" choice); +? returns Option[T] (the "the caller wants to
  handle the overflow case" choice). The ? operator at the call site then propagates the Option per the rules above.
  Rejected: language-level special-casing of Option and Result (the trait-based mechanism is more general and aligns
  with the principle that nothing in the language privileges specific stdlib types — any user type implementing Try
  participates equally); implicit cross-type ? (silently bridging Option and Result failure cases would require
  fabricating an error value from None or discarding an error value when going to Option; both lose information that
  should be explicit at the call site); if let sugar in v1 (the combination of match and ? covers the use cases; adding
  if let later if needed is non-breaking); making the ? operator dispatch on enum structure rather than a trait (would
  either hardcode Option/Result as special or require a generic enum-discrimination mechanism with the same complexity
  as the trait approach but less flexibility); treating traps as catchable values (trying to "recover" from overflow or
  division by zero would defeat their bug-finding purpose; the wrapping/saturating/checked operator variants exist
  exactly to handle those cases as values when that's the intent).
  Ready for the next topic? Topic 25: String and primitive non-numeric types.
  string is a built-in primitive type, language-level like i32. The compiler has direct knowledge of it; it is not a
  stdlib type with privileged literal syntax. This grants the language flexibility for compiler-level optimizations (
  small-string optimization, intern pools, constant folding of string literals at compile time per Topic 11) without
  dependency on a stdlib implementation. The lowercase string keyword is reserved (matching the lowercase convention for
  primitive types per Topic 7e and the naming-conventions decision).
  String literals follow grammar §2.5.5: plain strings ("..."), raw strings (r"...", r#"..."#, etc.), interpolation
  expressions ({expr} inside plain strings), and escape sequences (\n, \t, \xHH, \u{HHHHHH}, etc.). All produce values
  of type string.
  UTF-8 is the internal encoding. Strings are sequences of bytes interpretable as UTF-8; the type system guarantees
  every string value is valid UTF-8. No invalid-UTF-8 string can exist; constructors and conversions reject ill-formed
  input.
  Strings are opaque with respect to indexing — there is no s[i] operator. Direct indexing is rejected as a footgun:
  byte indexing produces meaningless results when an index lands mid-codepoint; char indexing is O(n) and silently hides
  the cost; both invite subtle bugs that surface only on non-ASCII input.
  Access to string contents uses explicit views:
* s.bytes() — view as a sequence of u8 values. Indexable, O(1) access, but operates on raw bytes (UTF-8-aware code must
  handle multi-byte sequences correctly).
* s.chars() — view as a sequence of char values (Unicode scalar values). Iterable; O(n) traversal.
* s.byte_len() — length in bytes. O(1).
* s.char_count() — number of Unicode scalars. O(n).
  The method names make the unit of measurement explicit at every call site. Users choose the appropriate view for their
  use case; the language does not pick a default that would be wrong for some workloads.
  Slicing uses explicit methods rather than range syntax:
* s.slice(start: isize, end: isize) -> string — char-boundary slicing. Start and end are char positions. Validates
  boundaries.
* s.byte_slice(start: isize, end: isize) -> string — byte-boundary slicing. Start and end are byte positions. Traps if
  the boundary lands mid-codepoint (which would produce invalid UTF-8).
  Both methods return a new string value. Invalid boundaries (mid-codepoint byte index, out-of-range positions) trap at
  runtime per Topic 10's trap-on-error philosophy.
  Strings are immutable, consistent with all bindings in the language per Topic 11's immutability principle. There is no
  in-place mutation. Every string operation that produces modified content returns a new string value. Stdlib operations
  like s.to_upper(), s.replace(old, new), s.trim(), s + other all return new strings. The runtime is free to share
  immutable backing storage between values, but this is an implementation detail invisible to the user.
  The + operator concatenates strings per Topic 5's operator framework. string fulfills Add with both operands and
  result typed as string. Concatenation:
  let greeting = "hello" + " " + "world"
  Interpolation is the preferred form when building strings from non-string values, per grammar §2.5.5:
  let label = "user {name} has {count} items"
  The interpolation expression {name} evaluates name and converts to string via the Display trait per the grammar's
  interpolation rule. Values whose types do not satisfy Display produce a compile error at the interpolation site.
  The complete set of primitive non-numeric types in the language:
* bool — Topic 16.
* char — Topic 16.
* string — this topic.
  No other non-numeric primitives. Byte sequences are u8[N] arrays per Topic 12. Other collection types (vectors, maps,
  sets) are standard library concerns built atop the language primitives. Other text-related types (UTF-16 strings,
  ASCII-only strings, byte strings with no encoding) are stdlib concerns if needed; the language commits to one string
  type, and that type is UTF-8.
  Rejected: stdlib string with privileged literal syntax (limits compiler-level optimization opportunities; the user
  explicitly preferred built-in); string[i] indexing (footgun under any semantics — byte-indexing produces mid-codepoint
  garbage, char-indexing hides O(n) cost); range syntax for slicing (would require defining a range type and slicing
  protocol; explicit methods are clearer about boundary semantics); mutable strings (contradicts the language's
  immutability principle); a single ambiguous length property (the byte-vs-char distinction matters and should be
  explicit at every use site); multiple string types (UTF-16, ASCII, etc.) in the language core (specialized
  representations belong in stdlib).

Topic 26: Tuples.
Tuples are structurally typed: two tuples with the same component types in the same order are the same type. (1, 2)
and (3, 4) are both (i32, i32). No type declaration is required to use a tuple type; the type expression (T1, T2, ...)
denotes the tuple type directly. This is the one structural-typing carve-out in an otherwise nominal type system,
justified by the fact that tuples are anonymous product types by design and carry no domain identity.
Field access uses numeric postfix syntax per grammar §3.15 (Postfix := '.' IntLit). Indices are zero-based and must be
integer literals:
let t = (1, "hello", 3.14)
let n = t.0 // i32
let s = t.1 // string
let f = t.2 // f64
Bounds checking happens at compile time. t.3 on a 3-tuple is a compile error. The index must be a literal integer —
runtime indexing with a variable expression (t.i) is not permitted, because tuple components can have different types
and the compiler must know the type of the accessed field statically.
Pattern destructuring follows grammar §3.14's TuplePat:
let (a, b, c) = (1, "hello", 3.14)
let (x, _, z) = some_tuple
let ((a, b), c) = ((1, 2), 3)
Tuple patterns appear in let bindings, match arms, and any other position where patterns are admitted. Nested tuple
patterns work to arbitrary depth. The wildcard _ ignores a component without binding it.
The unit type is (), with a single value also written (). Functions without a FinalExpr per grammar §3.13 return the
unit value implicitly. The unit type appears in pattern position as () to match unit-typed values, and as a type
expression for return types of functions producing no meaningful value.
The 1-tuple form requires a trailing comma to disambiguate from a parenthesized expression per grammar §3.15:
let single = (42,)         // 1-tuple of type (i32,)
let grouped = (42)         // i32 in parens — not a tuple
This trailing-comma convention is standard across languages with tuple support and resolves the syntactic ambiguity
cleanly.
Generic parameters appear in tuple types using standard generic syntax — no special mechanism. A function generic over a
tuple's component types is written using ordinary type parameters:
fn first[A, B](t: (A, B)) -> A:
t.0

fn swap[A, B](t: (A, B)) -> (B, A):
(t.1, t.0)
The tuple type (A, B) is a type expression like any other, and A and B are bound by the generic parameter list. Per
Topic 17's monomorphization rules, each unique tuple-type instantiation produces its own specialized code.
Variadic generics — abstraction over tuples of arbitrary arity — are not supported in v1. Functions generic over "any
tuple" would require either macro support or a different abstraction mechanism (e.g., a trait with associated types for
each component). Grammar §6 reserves higher-kinded types for future extension; variadic generics are in similar
territory and may be added later if usage patterns justify the complexity. For now, generic-over-tuple-component-types
covers the common case.
Trait conformance for tuples is supported via fulfill blocks per Topic 19b, subject to the orphan rule from Topic 15.
Since tuple types are structural and not declared in any module, the orphan rule's "trait or type defined locally" check
applies to the trait side only: a user can implement their own trait for any tuple shape, but cannot add a stdlib trait
to a tuple type from their own module.
fulfill MyTrait for (i32, string):
fn my_method(t: (i32, string)) -> ...
Stdlib provides fulfill blocks for common tuple types implementing common traits (Eq, Ord, Hash, Clone, Display, Debug).
Coverage extends through tuple arity 12 (matching the de facto industry convention from Rust and other languages).
Beyond arity 12, users implement explicitly. The arity limit reflects the practical observation that tuples larger than
12 components are rare in practice and typically indicate the user should be using a record instead.
Tuple-to-record conversion is explicit. Tuples are structural; records are nominal; they do not share identity, and the
compiler does not implicitly convert between them. Manual conversion uses field-by-field construction:
let t = (1.0, 2.0, 3.0)
let v = Vec3(x: t.0, y: t.1, z: t.2)
For ergonomic repeated conversion, a From impl per Topic 14 produces method-call conversion:
fulfill From[(f32, f32, f32)] for Vec3:
fn from(t: (f32, f32, f32)) -> Vec3:
Vec3(x: t.0, y: t.1, z: t.2)

let v: Vec3 = t.into::[Vec3]()
The orphan rule applies: the From impl can be written by the module owning Vec3 (the type side) or by the module owning
the source tuple type (which is no specific module, so this side never applies). In practice, From impls for
tuple-to-record conversion live in the module defining the record.
Rejected: nominal tuples (would require a type declaration for every tuple shape, defeating the purpose of tuples as
lightweight anonymous products); runtime-index field access (would require all tuple components to share a type,
contradicting tuples' role as heterogeneous products); range syntax or slicing on tuples (tuples are heterogeneous; "
slicing" has no meaningful semantics across different-typed components); variadic generics in v1 (significant complexity
addition for a use case adequately served by traits with specific tuple-arity impls); implicit tuple-to-record or
record-to-tuple conversion (would erode the structural-vs-nominal distinction at the core of the type system);
arity-unbounded stdlib trait impls (combinatorial explosion of compiler-provided impls; the arity 12 cutoff is pragmatic
and standard).

That completes the type-system topics. Locked: 1–26, with Topics 5, 7e, 9 amended per the grammar-conflict
reconciliation. The numeric system, record system, trait system, enum system, conversion system, newtype system,
intersection system, and string/tuple primitives are all settled.
Areas remaining outside the type system proper:

* Visibility and module model. Topic 20 covered name resolution; the public keyword and cross-folder visibility rules
  need fuller treatment.
* Error handling philosophy. Topic 24 covered Option/Result/?; broader error-handling conventions and panic semantics
  are adjacent.
* The reactive system. Signals, derived, the dependency graph — distinct axis.
* Nodes/connections beyond declaration. Placement syntax, runtime semantics, graph construction.

Topic 27: Visibility and module model.
Visibility is three-level: public, shared, private. Each keyword denotes a distinct scope; shared is the default and
requires no keyword.

* public — visible across package boundaries. Exported to external code that depends on this package.
* shared — visible within the same package (the module tree rooted at the package root). Default visibility; no keyword
  required. Equivalent to Rust's pub(crate).
* private — visible only within the declaring file. Sibling files in the same folder cannot see private declarations.
  A package is the unit of distribution — a project root or a named dependency. Within a package, all shared and public
  declarations are mutually visible per the module-graph reachability rules. Across packages, only public declarations
  cross the boundary; shared declarations are package-internal. The grammar's PathBase includes root for the current
  package's root and external dependency names for cross-package access.
  Module structure follows grammar §3.2: a folder is a module, and files within a folder are auto-mutually-visible
  without explicit use for shared and public declarations. Private declarations are file-scoped and not visible even to
  sibling files in the same folder. Cross-folder visibility requires explicit use statements with the appropriate
  visibility levels declared on the targets.
  The grammar's pub keyword is replaced throughout by the three-level model. Every position in the grammar that
  currently admits Pub? is updated to admit a visibility specifier per the three-level set; absence of a specifier means
  shared. The grammar revision propagates to all visibility-bearing productions (§3.4, §3.5, §3.6, §3.7, §3.8, §3.9,
  §3.10, §3.11). Topic 22's earlier note about renaming pub to public is subsumed by this broader revision.
  use imports carry visibility per the same three-level model. An unqualified use is shared; public use path::name
  re-exports the imported name with public visibility; private use path::name keeps the import file-local. This enables
  facade patterns — a module imports from internal sources and re-exports a curated subset with controlled visibility.
  Trait fulfill blocks per Topic 19b have no separate visibility specifier. An implementation is visible wherever both
  the trait and the type are jointly visible. If a caller can name Display and can name Person, the call
  person.display() resolves to the implementation; the visibility intersection is computed from the trait's and type's
  declarations, not from the fulfill block itself. Coherence per Topic 15 ensures at most one implementation exists
  per (trait, type) pair, so there is no ambiguity in which implementation is the visible one.
  Constructor visibility is independently controllable from type visibility per Topic 22e. The syntax uses a
  parenthesized modifier on the type visibility specifier:
  public type Email = string // type public, constructor public
  public(shared) type Email = string // type public, constructor shared
  public(private) type Email = string // type public, constructor private (smart-constructor pattern)
  shared(private) type SecretConfig = ... // type shared, constructor private
  The outer keyword is type visibility; the parenthesized inner keyword is constructor visibility. When the inner
  specifier is omitted, constructor visibility defaults to match type visibility. The inner specifier may never be more
  permissive than the outer specifier — private(public) is a compile error because a public constructor on a private
  type cannot be called from outside the type's visibility scope.
  Enum visibility applies uniformly to the enum type and all its variants. There is no per-variant visibility specifier.
  A public enum Foo exposes all variants publicly; a private enum Foo keeps the type and all variants file-local. Users
  who need a mix (some variants internal, some external) split the enum into multiple enums with different visibility
  levels.
  Record fields have independent visibility per grammar §3.5's TypeFieldDecl. Each field declares its own visibility:
  public type Account:
  public id: i64 // readable anywhere the type is
  email: string // shared (default) — readable within package
  private password_hash: string // readable only within this file
  Field visibility is independent from type visibility and constructor visibility. A field's visibility never exceeds
  its enclosing type's visibility — declaring a public field on a private type is a compile error for the same reason as
  the constructor case.
  Visibility interacts with Topic 20's uniform function call syntax dispatch through name resolution. A method call
  x.f() resolves f against names visible in the current scope. Visibility determines what is in scope:
* A private function is in scope only within its declaring file.
* A shared function is in scope within its declaring file and importable from any module in the same package via use or
  callable via path qualification.
* A public function is in scope as for shared, plus importable from any package depending on this one.
  The orphan rule from Topic 15 operates on the module-of-declaration, not on visibility. A fulfill block can satisfy
  the orphan rule regardless of its visibility level — visibility controls who can see and use the implementation, while
  the orphan rule controls where it can be declared.
  Rejected: two-level visibility (public / private only, Rust-style — too coarse; the package-internal-vs-external
  distinction is genuinely useful, particularly for library authors who want internal helpers visible across their
  package but not exposed to dependents); folder-scoped private (private means "this file" — finer-grained than folder
  is more useful in practice and matches the grain at which implementation details typically need to be hidden);
  separate visibility keyword for fulfill blocks (would create the case where a trait and type are both visible but the
  implementation isn't, leading to confusing "method not found" errors when the implementation clearly should exist);
  per-variant enum visibility (rare use case, splittable into multiple enums when needed, complexity not justified);
  allowing inner visibility specifier more permissive than outer (would create dead specifiers — a public constructor on
  a private type can never be called from outside the type's scope); using pub as the keyword (the user explicitly
  preferred public; grammar revision propagates).

Topic 28: Error handling philosophy.
The language uses a two-track failure model, locked in earlier topics and consolidated here:
Trap-track failures represent bugs and invariant violations: arithmetic overflow on default operators per Topic
10 (+, -, *, /, %, unary -), integer division by zero, out-of-range as casts, out-of-range array indices per Topic 12,
abs on signed-integer minimum per Topic 13, negative integer exponent on integer base per Topic 13, unwrap/expect on
Option::None or Result::Err per Topic 24, non-exhaustive match cases the compiler couldn't statically prove exhaustive,
runtime stack overflow, allocation failure, and explicit panic calls. Traps halt execution and produce diagnostics. They
are not catchable as values.
Value-track failures represent recoverable conditions and flow through the type system: Option[T] for failures carrying
no information beyond their occurrence, Result[T, E] for failures carrying contextual information, the ? operator per
Topic 24 for short-circuit propagation, the Try trait per Topic 24 dispatching ? to user-implementable types, the
From-conversion of failure types during propagation, the arithmetic operator variants (+?, -?, etc.) per Topic 10 for
producing Option-typed results from operations that would otherwise trap.
These tracks are not interchangeable. A trap does not become a Result::Err value; a Result::Err does not abort the
program. The user picks the mechanism based on the failure's nature when writing the code: traps for "this should never
happen if the program is correct"; Option/Result for "this might legitimately happen at runtime and the caller might
want to handle it."
The panic operation is a built-in function with signature fn panic(message: string) -> never. It triggers an immediate
trap with the given diagnostic message. The never return type allows panic to appear in expression position where any
type is expected, including inside match arms, conditional branches, and function bodies that return non-unit types:
let value = match maybe_value:
Some(x): x
None: panic("expected Some, got None")
The never type is a built-in primitive type with no values, written in lowercase consistent with other primitive type
keywords. It is the return type of functions that do not return normally — panic, infinite loops, functions that always
trap. The compiler treats never as unifiable with any type during type-checking: a value of type never can be used in
any context expecting any other type, because such a value can never exist at runtime. This is the "bottom type" or "
absurd type" from type theory, exposed as an ordinary primitive in the language's type system.
Trap behavior at runtime is process abort with diagnostic output. When a trap fires:

1. A diagnostic is printed including: the operation that triggered the trap (with operand values where available), the
   source location (file, line, column), and a stack trace through the call chain.
2. The process exits.
   There is no try/catch mechanism for traps. There is no way to recover from a trap as a value. There is no separate
   error channel beyond what flows through the standard type system via Option and Result.
   This is by design. Once a trap fires, the program exits. The only way to handle a failure recoverably is to use
   Result/Option from the start — and the operator/method variants in Topic 10 (+?, checked_add, etc.) make this choice
   available where overflow is a possibility. The user decides at the operation site whether a failure is a bug to be
   trapped or a condition to be handled. Choosing wrong at that point cannot be retroactively patched by a catch block;
   the language forces the decision upfront, which is the principal mechanism for keeping the two failure tracks honest.
   Diagnostic format for built-in traps includes the operation name and operand values where the runtime has access to
   them:
   panic: integer overflow: 2147483647 + 1
   at compute_total, src/billing.symphony:42:8
   called from main, src/main.symphony:7:3
   For user-triggered panic calls, the diagnostic includes the user-supplied message:
   panic: expected Some, got None
   at process_input, src/handler.symphony:24:10
   The format details are implementation-level; the semantic commitment is that diagnostics provide sufficient
   information for the user to identify what trapped, where, and through what call chain.
   The reactive system per Topic 11 uses the same two-track failure model. A trap inside a derived expression's
   computation propagates as a normal trap — the reactive system does not catch traps. A derived declaration whose
   expression has type Result[T, E] or Option[T] produces a reactive value of that type; consumers of the derived value
   handle the failure case using standard match or ? propagation. The reactive layer adds no special error mechanism
   beyond what already exists in the type system.
   Convention guidance for choosing between Option and Result: use Option[T] when the failure case carries no
   information beyond its occurrence (e.g., find_first(predicate) returns Option[T] because either the element exists or
   it doesn't — there's nothing to say about why it doesn't). Use Result[T, E] when the failure case carries information
   that the caller may want to inspect or react to (e.g., read_file(path) returns Result[bytes, IoError] because the
   caller often needs to know whether failure was due to missing file, permission denied, or transient I/O error). When
   in doubt, prefer Result — information about failure is rarely too much; absence of information makes debugging
   harder. This is convention, not a language rule; the compiler accepts either signature, and users choose based on
   what callers need.
   Rejected: trap-catching mechanism (try/catch for traps) — defeats the bug-detection purpose by enabling traps to be
   silently swallowed and turning runtime errors into normal control flow; structured exception handling — same problem
   at larger scale, plus the type system already provides the better mechanism via Result; configurable
   abort-vs-unwind (Rust's panic-strategy choice) — adds complexity for a choice that should be uniform, and the
   unwinding mode invites the abuse described; separate error channel for the reactive system — the two-track model
   already covers reactive failures cleanly; panic as a macro or special syntax — function-with-never-return-type is
   simpler and uses existing language machinery; uppercase Never — lowercase never matches the convention for primitive
   type keywords (i32, string, bool).
