use symphonyscript::syntax::ast::File;
use symphonyscript::syntax::parse;
use symphonyscript::Source;
use symphonyscript_fixtures::vocabulary;

fn file_of(text: &str) -> File<'static> {
    let vocab = vocabulary();
    let source = Source {
        path: "main.sym".into(),
        text: text.into(),
    };
    let parsed = parse(&source, &vocab);
    parsed.file().expect("a file")
}

fn main() {
    let _ = file_of("Clip c\n");
}
