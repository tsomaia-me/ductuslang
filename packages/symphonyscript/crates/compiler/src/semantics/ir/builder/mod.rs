#[cfg(test)]
mod bodies;
#[cfg(test)]
mod build_file;
#[cfg(test)]
mod cells;
#[cfg(test)]
mod expressions;
mod faults;
#[cfg(test)]
mod heads;
#[cfg(test)]
mod hir_builder;
#[cfg(test)]
mod index;
mod number_literal;
mod scalar_type;

#[cfg(test)]
pub(crate) use build_file::build_file;
pub(crate) use faults::Faults;
pub(crate) use number_literal::number_literal;
pub(crate) use scalar_type::scalar_type;

#[cfg(test)]
mod tests;
