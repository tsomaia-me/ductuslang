use symphonyscript::{Code, Compiler, Diagnostic, Failed};
use symphonyscript_core::{LimitOverrides, Vocabulary};

use crate::program::Program;

pub const EXPANSION_WARNING: &str = "the expansion yields more than";
pub const COUNTED_ITEMS: u64 = 1_000_000;

pub fn expanded_items(program: &Program, vocabulary: &Vocabulary) -> Result<Option<u64>, Failed> {
    let sources = program.compiler_sources();
    let warned = |below: u64| -> Result<bool, Failed> {
        let compiler = Compiler::with_limits(
            vocabulary,
            LimitOverrides {
                expanded_items: Some(COUNTED_ITEMS + 1),
                expanded_items_warning: Some(below),
                ..LimitOverrides::default()
            },
        )?;
        Ok(compiler.check(&sources).iter().any(warns_of_expansion))
    };
    if warned(COUNTED_ITEMS)? {
        return Ok(None);
    }
    let (mut low, mut high) = (0, COUNTED_ITEMS);
    while low < high {
        let middle = low + (high - low) / 2;
        if warned(middle)? {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    Ok(Some(low))
}

fn warns_of_expansion(diagnostic: &Diagnostic) -> bool {
    diagnostic.code == Code::LimitWarning && diagnostic.summary.starts_with(EXPANSION_WARNING)
}
