# Nodes, Parts, Connections, and Reactivity

This document specifies the language's composition layer: declarative
graphs of node instances linked by typed connections, with reactive
attributes propagating changes through the graph.

## The Reactive Model

The language has three reactive declaration kinds, distinguished by who
controls the value:

- **`signal name: Type = initial`** — a writable reactive cell. The
  initial value is supplied at declaration; subsequent values come from
  the runtime (mouse events, timers, network, etc.). User code reads
  signals; it does not write them through any source-level statement.
- **`attr name: Type = default`** — a per-instance writable reactive
  attribute on a node or connection. Conceptually a signal scoped to
  one instance: each instance carries its own cell. Like signals,
  attrs are written by the runtime, not by source-level assignment.
- **`derived name: Type = expr`** — a read-only reactive value defined
  by an expression. The runtime keeps it consistent with its
  dependencies: when any dependency changes, the expression
  re-evaluates and the new value propagates.

User code has no syntactic form for assigning to `signal`, `attr`, or
`derived` after declaration. Mutation of writable cells is performed
through the runtime's host-language API.

### Propagation

Reactivity propagates transitively through pure expressions. Any
expression that reads a signal, attr, or derived value is *reactive*;
pure functions called with reactive arguments produce reactive results;
constant expressions over reactive values are reactive. The compiler
tracks this provenance and produces diagnostics like *"value of `x` is
reactive because it depends on signal `mouse_position` at line 14"*.

Reactivity breaks compile-time evaluation. A reactive value cannot be
used where a compile-time-known value is required (e.g., in an array
length: `i32[some_signal]` is rejected).

### Pure vs reactive boundary

Source-level code never observes when the runtime re-evaluates a
`derived` or pushes new values to a `signal`. The user writes
declarative expressions; the runtime maintains consistency. The exact
timing, batching, and ordering of re-evaluation are runtime concerns
specified separately (deferred).

## Nodes

A node is a nominal type whose instances live in a reactive graph:

```
node Driver:
  satisfies Drivable, Insurable
  out: Drives, Owns
  attr expertise_level: i8
  attr risk_tolerance: f32 = 0.5
  derived skill_factor: f32 = self.expertise_level as f32 / 10.0
  derived aggressive: bool = self.risk_tolerance > 0.7
```

Body items:

- **`satisfies T1, T2`** — trait conformance. Same semantics as
  records.
- **`parts: T1, T2`** — types of child parts this node contains at
  placement time.
- **`in: T1, T2`** / **`out: T1, T2`** — types of incoming and outgoing
  connections this node participates in.
- **`attr`** declarations — per-instance writable reactive attributes
  on the node.
- **`derived`** declarations — per-instance read-only reactive values.

A node body **does not contain `fn` declarations**. Behavior on node
values comes from free functions whose first parameter is the node
type, callable via uniform call syntax (method-call, pipe-forward,
conventional).

## Parts

"Part" is a role, not a separate type. A part is a child node instance
contained inside a parent node. Parts appear via *placement* inside
the parent's placement body (see Placement below). The parent declares
the *types* of parts it accepts in its `parts:` clause; specific
instances appear at placement time.

## Connections

A connection is a directional link between two node instances, itself
a nominal type carrying reactive attributes:

```
connection Drives:
  from: Driver
  to: Drivable
  attr enhanced_handling: bool
  attr aggressiveness: f32 = 0.5
  derived effective_speed: f32 = to.top_speed * (from.expertise_level as f32 / 10.0)

connection Contains[T]:
  from: Container
  to: T
  attr index: usize
```

Body items:

- **`from: T`** (required, exactly once) — source endpoint type.
- **`to: T`** (required, exactly once) — destination endpoint type.
- **`attr`** and **`derived`** declarations — same semantics as for
  nodes.

A connection body does not contain `fn` declarations. Same rule.

Connections may be generic. Generic parameters scope over `from`,
`to`, and attr/derived declarations.

Inside a connection's `derived` expressions, `from` and `to` resolve to
the connection's endpoints — scope-local references to the two ends.

## The `self` Keyword

`self` is a context-restricted keyword. It is available **only inside
node and connection declaration bodies** — in `attr` default
expressions, `derived` expressions, and any expression evaluated within
the body. It resolves to the instance being declared or constructed.

```
node Driver:
  attr risk_tolerance: f32 = 0.5
  derived aggressive: bool = self.risk_tolerance > 0.7    // self inside node body
```

`self` is **not** available in:

- Record or enum bodies.
- Trait bodies (use capitalized `Self` for type-level receiver).
- Free functions, including those whose first parameter is a node.
  Such functions use the parameter name:

```
fn cautious(driver: Driver) -> bool:
  driver.risk_tolerance < 0.3       // parameter name, not self
```

References through `self` participate in the reactive dependency graph.
A `derived` expression reading `self.x` becomes reactive on changes to
`x`; the runtime re-evaluates whenever `x` changes.

## Placement

Placement is the syntax for *instantiating* nodes, parts, and
connections. It is distinct from value construction.

### Top-level instances

```
Driver john_doe:
  expertise_level: 10
  risk_tolerance: 0.8
  Drives/some_car | enhanced_handling: true | aggressiveness: 0.8
```

The first line is `TypeName instance_name:`. The body sets attributes
and declares child parts and connections.

### Setting attributes

A line `name: expr` sets the named attribute of the enclosing instance:

```
Driver john_doe:
  expertise_level: 10         // attr setting
  risk_tolerance: 0.8         // attr setting
```

### Child parts and connections

A line starting with a type name (no `:` immediately after the first
identifier) declares a child placement — either a child part or an
outgoing connection:

```
Component chip_b:
  label: "B"                              // attr setting
  Pin out1                                // child part (Pin)
    WiresTo/chip_a.in1 | resistance: 50  // outgoing connection from out1
    WiresTo/chip_a.in2 | resistance: 75
  Pin in1                                 // another child part
```

A line is an *attribute setting* if it has `: expr` immediately after
the first identifier. Otherwise it's a *placement*.

### Inline attribute pipes

After the type and optional instance name, additional attributes can be
set inline via `|`:

```
SomeConn | attr1: val | attr2: val | boolean_attr | !other_bool
```

Three forms:

- `| name: value` — set attribute to a value.
- `| name` — set boolean attribute to `true`.
- `| !name` — set boolean attribute to `false`.

### The `/` form for connections

A connection placement may specify its `to` endpoint inline using
`/expr`:

```
Drives/some_car | enhanced_handling: true
```

This places a `Drives` connection whose `to` is `some_car`. The `/`
form is the default-argument slot specific to connections.

## Trait Conformance

Nodes and connections use `satisfies` to declare trait conformance,
same as records. Trait methods are implemented as free functions:

```
node Driver:
  satisfies Displayable
  attr expertise_level: i8
  attr risk_tolerance: f32

fn display(d: Driver) -> string:
  "Driver(exp: {d.expertise_level}, risk: {d.risk_tolerance})"
```

Dispatch via uniform call syntax. Orphan rule applies normally.

## Deferred to Runtime Spec

The following are not specified at the language level:

- Exact evaluation timing and ordering of `derived` re-evaluation.
- Transactional batching and glitch avoidance.
- The host-language API for writing signals from outside.
- Memory representation of reactive cells and the dependency graph.
- Lifetime and ownership of node and connection instances across
  reactive updates.

Source-level code observes only that derived values remain consistent
with their dependencies and that `attr` writes propagate to all
transitively-derived consumers.