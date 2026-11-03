mod blocks;
mod calls;
mod constructs;
mod decls;
mod expressions;
mod finish;
mod ir_builder;
mod marks;
mod placements;
mod ranges;
mod types;
mod value;

pub(crate) use ir_builder::build_flat_file;
pub(in crate::semantics::ir::flat) use types::SCALARS;
