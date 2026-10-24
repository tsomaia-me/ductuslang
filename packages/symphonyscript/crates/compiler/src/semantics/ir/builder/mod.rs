mod bodies;
mod build_file;
mod expressions;
mod faults;
mod hir_builder;
mod index;
mod number_literal;
mod scalar_type;

pub(crate) use build_file::build_file;

#[cfg(test)]
mod tests;
