//! Graph-section ENTRIES: [`Scope`] and the cell/gate/wire/effect/stream
//! entries it holds — the structural mirror of the `.duir` text form's graph
//! section. The [`Module`](crate::module::Module) and its three section shells
//! live in `module/` instead (R55).
//!
//! The shape is IR_GRAMMAR §3, transcribed: `graph_section ::= 'graph' '{'
//! scope* '}'`, a `scope` carrying `entry*`, and an `entry` being one of five
//! forms. Every entry type mirrors its production clause for clause, in the
//! production's order.
//!
//! Nothing here validates. Which clauses a cell KIND admits, whether a path
//! resolves, whether a behavior handle is defined — all of it is the loader's
//! (F4, R21). The parser over-admits by design, so this is the shape of the
//! GRAMMAR and not the shape of a legal program.
//!
//! Two conventions run through the whole folder:
//!
//! - **Spans on named entries** (R17). A `Scope`, `Cell`, `Gate`, `Wire`,
//!   `Effect` and `Stream` each carry the span of their own line; the pieces
//!   inside them — a `Member`, a `Binding`, a `ReopenPair`, a `LookbackEntry` —
//!   do not, and a diagnostic about one points at the entry that holds it.
//! - **Empty vector = absent clause.** Where the grammar makes a set clause
//!   OPTIONAL, an empty vector is that clause's absence and the canonical
//!   rendering omits it. Where the clause is MANDATORY — a scope's `exposes`
//!   and `effects`, a gate's `inputs` and `guards`, an effect's `params` — an
//!   empty vector is a written `[]`.

pub mod binding;
pub mod binding_target;
pub mod cell;
pub mod cell_kind;
pub mod effect;
pub mod entry;
pub mod gate;
pub mod lookback_entry;
pub mod member;
pub mod member_driver;
pub mod provenance;
pub mod reopen_pair;
pub mod scope;
pub mod stream;
pub mod stream_policy;
pub mod value_literal;
pub mod wire;
pub mod wire_to;

pub use self::binding::Binding;
pub use self::binding_target::BindingTarget;
pub use self::cell::Cell;
pub use self::cell_kind::CellKind;
pub use self::effect::Effect;
pub use self::entry::Entry;
pub use self::gate::Gate;
pub use self::lookback_entry::LookbackEntry;
pub use self::member::Member;
pub use self::member_driver::MemberDriver;
pub use self::provenance::Provenance;
pub use self::reopen_pair::ReopenPair;
pub use self::scope::Scope;
pub use self::stream::Stream;
pub use self::stream_policy::StreamPolicy;
pub use self::value_literal::ValueLiteral;
pub use self::wire::Wire;
pub use self::wire_to::WireTo;
