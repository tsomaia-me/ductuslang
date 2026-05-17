# Inline Attribute Pipes — Complete Spec

## Surface forms

Inside a placement (top-level or nested), after the `TypeRef` and the
optional instance-name identifier, zero or more *attribute pipe*
clauses may follow on the same line. Each pipe is introduced by a
literal `|` character. Three syntactic forms:

```
| name: value      -- set attribute `name` to the expression `value`
| name             -- set boolean attribute `name` to true
| !name            -- set boolean attribute `name` to false
```

## Grammar

```
PlacementInline := DefaultArgPart? AttrPipe*
AttrPipe        := '|' AttrPipeBody
AttrPipeBody    := Ident ':' Expr      -- value-set form
                 | '!' Ident           -- boolean-false form
                 | Ident               -- boolean-true form
```

The parser distinguishes the forms by what follows the identifier
after `|`:

- `Ident` followed by `:` → value-set; the `Expr` after `:` is parsed
  as an ordinary expression.
- `!` followed by `Ident` → boolean-false.
- `Ident` not followed by `:` → boolean-true.

The parser uses a one-token lookahead after the `Ident` to commit. The
`Expr` in the value-set form is a full expression, but it terminates at
the next `|` or at the end of the placement line — pipes are
left-associative on the surface line.

## Semantics

Each `AttrPipe` sets exactly one attribute of the instance being
placed. The attribute must:

1. Be declared on the type being placed (directly via `attr` on the
   node or connection type, or inherited via any trait the type
   satisfies that contributes attrs to the type's effective surface).
2. For value-set form: be of any type. The expression's type must
   match the attribute's declared type, subject to the standard
   widening and conversion rules.
3. For boolean-true and boolean-false forms: be of type `bool`. A
   non-boolean attribute used with the bare `| name` or `| !name` form
   is a compile error.

The order of pipes on a placement line is significant for evaluation
order of the expression side; the resulting attribute values do not
depend on pipe order because each pipe targets a distinct attribute
name. Setting the same attribute name via two pipes on one placement
is a compile error (duplicate-set, same diagnostic class as duplicate
struct-field arguments).

## Interaction with the placement body

A placement may set attributes via inline pipes, via the placement
body's `name: expr` lines (attribute settings), or both. The two
mechanisms target the same underlying attributes; setting an attribute
both inline and in the body is a compile error.

```
Sensor s1 | gain: 0.5 | active:
  gain: 0.8                            -- ✗ duplicate: gain already set inline
  threshold: 0.1                       -- ✓ first set
```

The convention is inline pipes for short attribute lists (one or two
values), placement body for longer ones. There is no hard rule.

## Whitespace

The `|` token is whitespace-tolerant on both sides. The only
constraint is that the entire pipe (from `|` through the end of its
`AttrPipeBody`) appears on a single placement line — pipes do not
span line boundaries.

A pipe at the end of a placement line, followed by an indented
placement body on subsequent lines, is well-formed:

```
Driver alice | active | role: "supervisor":
  expertise_level: 10
  Drives/car1
```

## Interaction with flags

A placement's `TypeRef` may carry single-character *flags* immediately
adjacent to the type name (no whitespace). Flags appear before any
optional instance name and before the first pipe:

```
Pin' p1 | direction: In
^^^^                              -- TypeRef with flag '
     ^^                           -- instance name
        ^^^^^^^^^^^^^^^^^^^^      -- inline pipe
```

Flags resolve to specific boolean attributes via the type's trait-level
flag mapping. A flag is equivalent to the boolean-true pipe form
`| name` where `name` is the attribute aliased by the flag character —
but flags are positionally adjacent to the type and pipes are at the
end of the placement line. Mixing both for the same underlying
attribute (the flag form and a pipe form referring to the same
attribute) is a compile error: duplicate-set.

## Examples

```
-- Connection placement, two values + one boolean:
Drives/car1 | enhanced_handling: true | aggressiveness: 0.8

-- Node placement, mix of boolean-true and boolean-false:
Sensor s1 | active | !calibrated

-- Pipe with computed expression:
PowerStage ps | voltage: base_voltage * scale_factor + offset

-- Pipe with reactive expression (the placement happens once;
-- the resulting attribute is reactive):
Driver d | risk_tolerance: derived_risk_for(user_profile)
```

---

# Connection `/` Form — Complete Spec

## Surface form

A connection placement may specify its `to` endpoint inline using a `/`
immediately following the type name (and any flags), before the
optional instance name and before any pipes:

```
Drives/some_car
WiresTo/chip_a.in1 | resistance: 50
Contains/[i32]/array_buf | index: 0
```

The `/Expr` is the `DefaultArgPart` of the placement's inline section.

## Grammar

```
PlacementInline := DefaultArgPart? AttrPipe*
DefaultArgPart  := '/' Expr
```

The expression after `/` is parsed as an ordinary expression,
terminating at the first `|` or at the end of the placement line.

## Semantics

The `/Expr` slot supplies the `to` endpoint of a connection placement.
The form is permitted only on placements of *connection* types; using
`/Expr` on a node placement is a compile error.

The expression must:

1. Be of a type that satisfies the connection's `to` clause type
   (`connection Drives: to: Drivable` means `/Expr` must produce a
   value of type satisfying `Drivable`).
2. Reference a value or instance reachable in the placement's scope.
   Common patterns: a sibling instance name, a path into a parent's
   parts (`chip_a.in1`), an expression returning a target.

## Equivalence to attribute-style setting

The `/Expr` form is syntactic sugar for setting the connection's
implicit `to` attribute. The following are equivalent in effect:

```
Drives/some_car | enhanced_handling: true       -- sugar form

Drives:                                          -- desugared
  to: some_car
  enhanced_handling: true
```

The sugar form is the conventional choice. The desugared form is
sometimes used when `to` is a complex expression that reads better on
its own line, or when the placement has many other attribute settings.

## One per placement

A connection placement may have at most one `/Expr` slot. Two `/Expr`
slots on the same placement is a parse error.

Setting `to` both via `/Expr` and via the placement body (`to: expr`)
is a compile error — duplicate-set, same diagnostic class as inline-pipe
duplicates.

## Not available on node placements

Node placements do not have a `to` slot. The `/Expr` syntax is not
defined for them; the parser rejects `NodeType/expr` with a diagnostic
identifying that `/Expr` is connection-only.

## Interaction with flags and pipes

The order of inline parts on a connection placement is fixed:

```
TypeRef Flags? InstanceName? DefaultArgPart? AttrPipe*
```

- Flags are adjacent to `TypeRef` (no whitespace).
- An optional instance name follows (whitespace-separated).
- `DefaultArgPart` (`/Expr`) follows the name.
- Attribute pipes (`|`) come last.

```
WiresTo' my_wire / chip_b.in1 | resistance: 50 | reverse_polarity
^^^^^^^^                                              -- TypeRef + flag
         ^^^^^^^^                                     -- instance name
                  ^^^^^^^^^^^^                        -- default arg slot
                               ^^^^^^^^^^^^^^^        -- pipe 1
                                                ^^^^^^^^^^^^^^^^^  -- pipe 2
```

The instance name is optional; an unnamed connection placement
(connections rarely need names) is written:

```
WiresTo/chip_b.in1 | resistance: 50
```

## Examples

```
-- Simple connection to a sibling instance:
Driver alice:
  Drives/car1

-- Connection with attributes:
Component chip_b:
  Pin out1
    WiresTo/chip_a.in1 | resistance: 50
    WiresTo/chip_a.in2 | resistance: 75

-- Connection whose target is a method-call expression:
Sensor s1:
  Sends/find_target_for(self.id) | priority: high

-- Generic connection with explicit type argument and default-arg slot:
Container vec_buf:
  Contains::[i32]/array_element | index: 0
```

## Why this exists

The `to` endpoint is the most-set attribute on connections — every
connection has one and the syntax is cluttered without a dedicated
slot. The `/Expr` form factors out this common case into a positional
slot adjacent to the type name, leaving pipes free for the
characteristically-attribute attributes (`resistance`, `priority`,
etc.) and giving connection placements a recognizable shape.

It didn't include the full Flags spec — only brief mentions of how flags interact with pipes. Here's the complete spec.

---

# Flags — Complete Spec

## What flags are

A *flag* is a single non-letter character that appears adjacent to a
`TypeRef` in a placement and aliases a boolean attribute of the placed
type. Flags are a concision mechanism: in domains with frequently-set
boolean attributes (electronic pin polarity, signal direction, port
type, etc.), single-character flag tokens make placements scan as
declarative diagrams rather than dense attribute lists.

```
Pin' p1                          -- ' flag set on Pin
Pin'! p1                         -- two flags: ' and !
Wire/chip_a.out | reverse_polarity: true       -- equivalent without flag
Wire'/chip_a.out                                -- equivalent with flag (if ' maps to reverse_polarity)
```

## Surface form

A flag is a single character chosen from a restricted set, written
immediately after a `TypeRef`'s path with no intervening whitespace.
Zero or more flags may follow contiguously. Once whitespace appears,
the flag run ends.

```
TypeRef := Path FlagsRun?
FlagsRun := FlagChar+
FlagChar := one of: ' ! ? * + ^ ~ @ $ #
```

The exact set of permitted flag characters is restricted to symbols
that are not part of identifiers and not used by other syntactic forms
in placement position. Of these, `'`, `!`, `?`, and `*` are the
conventional choices; others are reserved for domain-specific
extensions.

The flag run is a contiguous, ordered sequence. `Pin'!` is two flags
in order: first `'`, then `!`. The order is not semantically
significant for attribute-setting purposes (each flag targets a
distinct attribute), but order is preserved in source for
readability.

## Declaration: `@flag` annotation

A boolean `attr` declaration on a node, connection, or trait may carry
an `@flag` annotation specifying the flag character that aliases it:

```
node Pin:
  @flag('!')
  attr reverse_polarity: bool = false

  @flag('\'')
  attr is_power: bool = false

trait HasDirection:
  @flag('*')
  attr is_starred: bool
```

The annotation argument is a character literal — a single character in
single quotes per the language's `char` literal syntax (§9.1.2).

The boolean attribute and its flag are coupled: setting the flag at a
placement site is equivalent to setting the attribute to `true`. The
attribute is the primary declaration; the flag is the secondary
alias.

Only boolean attributes (type `bool`) may have `@flag`. A non-boolean
attribute with `@flag` is a compile error.

## Flag-character uniqueness

Within a type's effective attribute surface (its own `attr`
declarations plus those inherited via satisfied traits), each flag
character must be unique. Two attributes claiming the same flag
character is a compile error at the type's declaration site, with the
diagnostic identifying both attributes.

```
trait HasFlag1:
  @flag('!')
  attr a: bool

trait HasFlag2:
  @flag('!')
  attr b: bool

node Bad:                        -- ✗ both inherited flags claim '!'
  satisfies HasFlag1, HasFlag2
```

This conflict is resolved at the type-declaration site (parallel to the
overlapping-method-name rule from §3.2.1). The fix is to drop one of
the satisfies, rename one of the attributes (which requires a different
flag character), or wrap one trait via a newtype.

## Use: placement-site resolution

At a placement site, each flag character in the `FlagsRun` resolves to
the boolean attribute it aliases in the placed type's effective
attribute surface. Each flag sets its target attribute to `true`.

```
node Pin:
  @flag('!')
  attr reverse_polarity: bool = false
  @flag('*')
  attr is_starred: bool = false

-- Placement:
Pin'!* p1

-- Equivalent to:
Pin p1 | reverse_polarity: true | is_starred: true

-- Or in body form:
Pin p1:
  reverse_polarity: true
  is_starred: true
```

A flag character not declared on the placed type's effective surface is
a compile error with a diagnostic listing the type's available flags.

## No flag for setting `false`

The flag form sets a boolean attribute to `true`. There is no flag form
for setting `false`; boolean attributes default to `false` (or to
whatever the `@flag`-annotated attribute's default is), and the user
either uses the flag to opt in or omits it to retain the default.

If a default-`true` attribute needs to be set to `false`, the user
uses the explicit pipe form `| !name`:

```
node Output:
  @flag('?')
  attr enabled: bool = true               -- default true

Output' out1                              -- redundant: ? sets enabled to true (already true)
Output | !enabled                         -- correct: set enabled to false
```

The asymmetry — flags set `true` only — is deliberate. Flags are for
the *unusual* case in a domain (the boolean default chosen so most
instances don't need the flag). When the unusual case becomes more
common, the default should be flipped at the declaration site, not
worked around at every call site.

## Disambiguation: `'` as flag vs character-literal opener

The character `'` is both a flag character and the opener of a `char`
literal (§9.1.2). Disambiguation is positional:

- **After a TypeRef path in placement position**, `'` opens a flag run
  if no whitespace separates it from the path.
- **In expression position** (everywhere else, including attribute
  values), `'` opens a character literal.

```
node Pin:
  @flag('\'')                    -- char literal in annotation
  attr is_marked: bool

Pin' p1                          -- flag (no whitespace after Pin)
Pin p1 | label: 'a'              -- char literal in attribute value
```

The grammar's lookback rules formalize this disambiguation. A
single-quote character immediately following an identifier or path
(no intervening whitespace) in a placement context is a flag-run
opener; in any other position it is a char-literal opener.

## Interaction with other inline parts

A placement's inline parts have a fixed order:

```
TypeRef FlagsRun? InstanceName? DefaultArgPart? AttrPipe*
```

Flags come immediately after the type name. The instance name (if
present) follows. The default-arg slot (`/Expr`, for connections) and
the pipes (`|`) come after the name.

```
WiresTo'! my_wire / chip_b.in1 | resistance: 50
^^^^^^^^                                              -- TypeRef + 2 flags
         ^^^^^^^^                                     -- instance name
                  ^^^^^^^^^^^^                        -- default-arg slot
                               ^^^^^^^^^^^^^^^        -- pipe
```

## No duplicate-set across forms

A boolean attribute may be set via at most one mechanism per placement:
the flag form, the inline pipe form (`| name` or `| !name`), or the
placement body form (`name: expr` line). Two mechanisms targeting the
same attribute is a compile error.

```
Pin' p1 | reverse_polarity: false             -- ✗ duplicate: ' flag and pipe both target reverse_polarity

Pin p1 | is_starred:
  is_starred: false                            -- ✗ duplicate: pipe and body both target is_starred
```

This rule parallels the duplicate-set rule for attribute pipes; the
diagnostic class is the same.

## Examples

```
-- A type with two flag-aliased booleans:
node Component:
  @flag('?')
  attr is_optional: bool = false
  @flag('*')
  attr is_critical: bool = false
  attr label: string

-- Placement variants:
Component c1 | label: "C1"                       -- both flags default to false
Component? c2 | label: "C2"                       -- is_optional: true
Component* c3 | label: "C3"                       -- is_critical: true
Component?* c4 | label: "C4"                      -- both true

-- Equivalent forms for the last one:
Component c4 | is_optional: true | is_critical: true | label: "C4"
Component c4:
  is_optional: true
  is_critical: true
  label: "C4"
```

## What flags exist for

Flags trade an alphabetic-attribute-name for a single-character token.
They are appropriate when:

- The attribute is *boolean* and *frequently set*.
- The domain naturally uses single-character markers (pin polarity,
  port direction, etc.).
- The flag character is meaningful by convention in the domain (`'` for
  primed/derived variables, `!` for negated/inverted state, `*` for
  starred/special).

Flags are *inappropriate* when:

- The attribute is rarely set — the pipe form is fine.
- No clear single-character convention exists in the domain — flag
  choice would be arbitrary and reduce readability.
- The boolean is not the "unusual case" — defaults should be set so the
  flag is the rare opt-in, not the common case.

In domains without strong single-character conventions (general
business logic, configuration), prefer the pipe form `| name` over
introducing flags.
