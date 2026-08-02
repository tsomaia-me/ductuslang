//! Diagnostics: `Span` (byte offsets), the `DiagClass` registry, and the
//! `Diagnostic` carrier that flows through every stage's `Result`.
//!
//! `Span` lives here because diagnostics are its universal consumer; the text
//! layer's tokens carry this same type rather than defining their own (R32).

pub mod class;
pub mod diagnostic;
pub mod span;

pub use self::class::DiagClass;
pub use self::diagnostic::Diagnostic;
pub use self::span::Span;
