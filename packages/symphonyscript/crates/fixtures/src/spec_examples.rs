use crate::document_examples::document_examples;
use crate::spec::SPEC;
use crate::spec_example::SpecExample;

pub fn spec_examples() -> Vec<SpecExample> {
    document_examples(SPEC)
}
