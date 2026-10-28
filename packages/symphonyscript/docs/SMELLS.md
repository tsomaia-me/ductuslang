# Smells

The defects that recur in SymphonyScript's code, each with its bad form and its good form from this codebase. An implementer writes none of them; every reviewer checks the diff for each. Members are entries of the ledger, `.memory/symphonyscript/CARRY.md`.

## Classes

**A classifier with a catch-all arm.** Name every variant in a `match` that answers a question about an expression, so a form nobody decided fails to compile, because a `_ => {}` arm answers "no" for every form it does not list, and each unlisted form surfaces as a defect of its own. Members: 132, 243, 248, 258, 277.\
**Bad** (`resolve/reads_cell.rs` `measures_variable_cell`, `731f9da`; the arm stands on main)
```rust
ExprKind::Binary(BinaryOp::Add, lhs, rhs) => {
    stack.extend([(*lhs, top_level), (*rhs, top_level)]);
}
ExprKind::Slice { base, .. } => stack.push((*base, top_level)),
_ => {}
```
**Good** (`infer/positions.rs` `arm_position`)
```rust
fn arm_position(kind: &ExprKind, position: usize) -> bool {
    match kind {
        ExprKind::If(..) => position > 0,
        ExprKind::Match { arms, .. } => {
            position > 0 && (position.is_multiple_of(2) || position > 2 * arms.len())
        }
        ExprKind::Error
        | ExprKind::Number(..)
        | ExprKind::Bool(_)
        | ExprKind::Str(_)
        | ExprKind::Name(_)
        | ExprKind::Variant { .. }
        | ExprKind::To
        | ExprKind::Convert(..)
        | ExprKind::Array(_)
        | ExprKind::Field(..)
        | ExprKind::Index(..)
        | ExprKind::IndexFromEnd(..)
        | ExprKind::Slice { .. }
        | ExprKind::History { .. }
        | ExprKind::Call(..)
        | ExprKind::Unary(..)
        | ExprKind::Binary(..) => false,
    }
}
```

**A construct that misses a consumer.** Carry each construct or type property a milestone adds through every consumer that exists, and test each: an `if` or `match` arm (chosen and passed over, compile-time and runtime), a join, a slice, `len`, `bytes`, an index, a `for` iterable, an attribute set and unset, an annotation, a native argument and result, a history read of each cell kind, an inlined, recursive and bounded call, a `to` read, and code nothing expands, because a property threaded through the consumers its author thought of is wrong in the rest. Members: B1-R12, B1-RW3, B2-H1, B2-H2 (a history read has its cell's type, width, kind and layout); B2-R1 to B2-R3, B2-H3; the arms of a choice: 225, 244, 249, 259, 278.\
**Bad** (`e791da6`: draws nothing, the read taking its fallback's kind)
```sym
input x = 1.5
let y = x * 2
state s = y{1, 0}
Clip c:
    Note/(s)
```
**Good** (`tests/capacities.rs` `a_history_read_has_its_cells_kind_for_an_input_a_let_and_a_state`)
```rust
(
    &[
        "input x = 1.5\n",
        "let y = x * 2\n",
        "state s = y{1, 0}\n",
        "Clip c:\n    Note/(s)\n",
    ],
    &[(Code::FloatToInteger, "s")],
),
(
    &[
        "input x = 1.5\n",
        "let y = x * 2\n",
        "state s = y{1, 0}\n",
        "Clip k | gain=s:\n    Note/(i32(s))\n",
    ],
    &[],
),
(
    &[
        "input x = 1.5\n",
        "state f = x + 1\n",
        "state s = f{1, 0}\n",
        "Clip c:\n    Note/(s)\n",
    ],
    &[(Code::FloatToInteger, "s")],
),
```

**A fault beside a fault.** Draw beside a fault exactly what ARCHITECTURE §7's placeholder rule draws, and test a name placeholder against its twins, because a walk that stops at a placeholder hides independent faults, and a walk that flows the position's type past it draws faults no value of the name draws. Members: 239, 246, 252, 253, 257, 263, 266 to 274.\
**Bad** (`0d52c37`: draws unknown-name alone)
```sym
Clip c:
    Note/(nothing + (1 + true))
```
**Good** (`tests/typing.rs` `every_fault_beside_another_fault_draws_by_the_placeholder_rule`)
```rust
if expression.holds_name() {
    let mut expected = common_to_twins(compiler, &program);
    for site in sites.iter().filter(|site| site.leaf == NAME) {
        expected.insert((
            Code::UnknownName,
            "main.sym".into(),
            base + site.start as u32,
            base + site.end as u32,
        ));
    }
    if found != expected {
        failures.push(format!(
            "{program}\n  found {found:?}\n  twins {expected:?}"
        ));
    }
    continue;
}
```

**A walk that repeats its work.** Memoize a walk's answers per compilation, stamped by what invalidates them, and keep what a walk learned before it blocks, because a fresh map per call repeats a shared walk for every reader, and a walk that discards its work on a miss repeats it for every later declaration. Members: B1-HW3, B1-HW10, B1-HW14, B2-H6, B2-H8.\
**Bad** (`infer/capacities.rs`, `e791da6`)
```rust
fn extent_with(&mut self, root: NodeId, env: &Env) -> Result<Extent, Internal> {
    let mut done: FastMap<NodeId, Extent> = FastMap::default();
    self.extent_into(root, env, &mut done)?;
    done.remove(&root).ok_or(Internal::missed("capacity"))
}
```
**Good** (`infer/capacities.rs` `extent_with`, and `extent_into`'s lookup)
```rust
fn extent_with(&mut self, root: NodeId, env: &Env) -> Result<Extent, Internal> {
    let stamp = (self.memo.settle_epoch, self.memo.binding_layouts.len());
    if self.memo.extent_stamp != stamp {
        self.memo.extent_cache.clear();
        self.memo.extent_stamp = stamp;
    }
    let mut done: FastMap<NodeId, Extent> = FastMap::default();
    self.extent_into(root, env, &mut done)?;
    done.remove(&root).ok_or(Internal::missed("capacity"))
}
```
```rust
if let Some((variable, capacity)) = self.memo.extent_cache.get(&node) {
    let cached = if *variable {
        Extent::Variable(capacity.clone())
    } else {
        Extent::Fixed(capacity.start().floor())
    };
    done.insert(node, cached);
    continue;
}
```

**An eager compile-time array.** Read a compile-time join or slice through its operands and bounds in every consumer (a `for`, a slice, `len`, an index, a fit check), because one consumer that builds the whole range builds every element the lazy form avoids. Members: B1-RW17, B1-RW19, B1-RW21.\
**Bad** (`evaluation/arrays.rs`, `7dbba62`)
```rust
Ok(Some(CVal::Array(CArray::new(array.range(start, end)))))
```
**Good** (`evaluation/arrays.rs`)
```rust
Ok(Some(CVal::Array(CArray::slice(&array, start, end))))
```

**A help that names a refused edit.** Choose a help by the value's type, and test it by applying the code it quotes and compiling again, because a help that reads well can name an edit the compiler refuses. Members: B1-R21, B1-RW9, B1-RW16, B2-R9, B2-RW8, B2-RW14.\
**Bad** (`e791da6`, for `input x: u8[..2] = "GET"`)
```text
slice the value to its newest 2 elements with `[^2..]`
```
**Good** (`infer/array_checks.rs`)
```rust
pub(in crate::semantics::analysis) fn capacity_fix(
    &self,
    value: crate::semantics::analysis::ids::NodeId,
    (capacity, holds): (u32, u32),
    annotated: bool,
) -> Result<String, Internal> {
    if !self.is_string_bytes(value)? {
        return Ok(Self::capacity_help(capacity));
    }
    Ok(Self::string_capacity_help(capacity, holds, annotated))
}
```

**A flow that stops in the wrong place.** Test each operator with a fixed operand that saturates in its own type, at a position the expected type reaches and at each place SPEC §4.2 stops it (a comparison's operands, an index, a float `div`'s operands, a recursive argument), because the flow is one rule over every operator and every stop, and a row tests one of each. Members: G4, B1-RW1, B1-RW7, 261.\
**Bad** (`e791da6`: draws does-not-fit at `400`)
```sym
Clip c:
    Note/(u8(250) * 2 - 400)
```
**Good** (`analysis/addition_tests.rs` `a_fixed_type_operand_widens_and_the_literals_beside_it_take_the_flowing_type`)
```rust
for (read, pitch) in [
    ("u8(250) * 2 - 400", 100),
    ("u8(200) + 100", 300),
    ("u8(250) + 300", 550),
] {
```

**A length past its limit.** Check each limit at its edge, one past it and far past it, at every construct that grows a length (a join, a slice bound, a state's iteration, an attribute, a native), and convert with `try_from`, because a length grows at each construct separately and `as` truncates without a word. Members: B1-H4, B1-HW1, B1-R19, B1-RW17, B2-R13.\
**Bad** (`vocab/vocabulary_builder.rs`, `e791da6`)
```rust
let id = NativeId::new(self.natives.len() as u32);
```
**Good** (`vocab/vocabulary_builder.rs`)
```rust
let id = NativeId::new(
    self.token,
    u32::try_from(self.natives.len()).unwrap_or(u32::MAX),
);
```

## Patterns

**Growth without a bound.** Check a bound while the collection grows, because a check after collecting allocates without limit first. Member: B2-R12.\
**Bad** (`core/src/types/bounded.rs`, `e791da6`)
```rust
pub fn from_items(items: impl IntoIterator<Item = T>) -> Option<Self> {
    let items: Vec<T> = items.into_iter().collect();
    (items.len() <= N).then_some(Self { items })
}
```
**Good** (`core/src/types/bounded.rs`)
```rust
pub fn from_items(items: impl IntoIterator<Item = T>) -> Option<Self> {
    let mut held = Vec::new();
    for item in items {
        if held.len() == N {
            return None;
        }
        held.push(item);
    }
    Some(Self { items: held })
}
```

**Derived equality over floats.** Compare floats bit for bit through `Value`, because derived `PartialEq` makes NaN unequal to itself and `-0.0` equal to `0.0`, so a memo, a dedup or a change check misfires. Member: B2-R12.\
**Bad** (`core/src/types/bounded.rs`, `e791da6`)
```rust
#[derive(Clone, PartialEq, Debug)]
pub struct Bounded<T, const N: usize> {
    items: Vec<T>,
}
```
**Good** (`core/src/types/bounded.rs`)
```rust
impl<T: ArrayElement + Copy, const N: usize> PartialEq for Bounded<T, N> {
    fn eq(&self, other: &Self) -> bool {
        let values = |items: &[T]| {
            let mut values: Vec<Value> = Vec::new();
            for item in items {
                item.push_elements(&mut values);
            }
            values
        };
        self.items.len() == other.items.len() && values(&self.items) == values(&other.items)
    }
}
```

**Cascading recovery.** Lower what a syntax fault touches to an error placeholder, because a guessed type reports errors the user never made. Members: B2-R7, B2-R8, B2-RW6, 236, B1-RW23.\
**Bad** (`e791da6`: draws type-syntax and type-mismatch)
```sym
input g: i32[..] = [1, 2]
Clip c:
    Note/(g[0])
```
**Good** (`tests/capacities.rs` `a_type_whose_length_or_capacity_is_missing_or_malformed_is_a_placeholder`)
```rust
(
    &["input g: i32[..] = [1, 2]\n", read],
    &[(Code::TypeSyntax, "]")],
),
```

**Span drift.** Draw a diagnostic where its twins draw it, because a diagnostic placed on a part of the fault points the user at code that is right. Members: B1-RW10, B2-RW11, 278.\
**Bad** (`731f9da`: the arm-shape mismatch lands at `f(0)`)
```sym
input flag = false
fn f(n): if n > 3 then [1] else [1] + f(n + 1)
Clip c:
    Note/((if flag then f(0) else [[1]])[0][0])
```
**Good** (`tests/capacities.rs` `the_arms_of_a_choice_are_type_checked_against_one_another_whatever_chooses`)
```rust
(
    &[
        flag,
        "fn f(n): if n > 3 then [1] else [1] + f(n + 1)\n",
        "Clip c:\n    Note/((if flag then f(0) else [[1]])[0][0])\n",
    ],
    &[(Code::TypeMismatch, "if flag then f(0) else [[1]]")],
),
```

**A clause no test isolates.** Give each error rule a row holding one fault, and each condition of a rule a row that fails when that condition alone is deleted, because a row with two faults passes when either rule breaks. Members: B1-R13, B2-R10.\
**Bad** (`tests/error_rules.rs`, `e791da6`: an integer at a string parameter beside the cell-reading argument)
```rust
"input i = 0\nClip c:\n    Note/(text_length(i))\n",
```
**Good** (`tests/error_rules.rs`)
```rust
natives(
    "10.1",
    "compile-time-native",
    "input n = 0\nClip c:\n    Note/(tag(\"abc\", n))\n",
),
```
