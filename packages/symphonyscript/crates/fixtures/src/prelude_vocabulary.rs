use symphonyscript_core::{Count, Vocabulary};

pub fn prelude_vocabulary(text: &str) -> Vocabulary {
    let mut v = Vocabulary::builder("preluded");
    let clip = v.node("Clip");
    v.kind(clip).top_level(Count::Any);
    v.prelude("prelude.sym", text);
    v.build().expect("the vocabulary builds")
}
