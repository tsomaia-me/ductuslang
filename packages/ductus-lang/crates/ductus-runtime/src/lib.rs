//! Ductus runtime kernel — loads a graph module, stores cell state, runs the
//! write→commit→read loop. Strictly sequential in M0.

pub mod engine;
pub mod loader;
pub mod provider;
pub mod store;
pub mod trace_sink;
pub mod value;
