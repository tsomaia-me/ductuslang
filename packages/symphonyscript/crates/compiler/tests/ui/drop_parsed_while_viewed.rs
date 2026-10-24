use symphonyscript::syntax::parse;
use symphonyscript::Source;
use symphonyscript_fixtures::vocabulary;

fn main() {
    let vocab = vocabulary();
    let source = Source {
        path: "main.sym".into(),
        text: "Clip c\n".into(),
    };
    let parsed = parse(&source, &vocab);
    let root = parsed.root();
    drop(parsed);
    println!("{root}");
}
