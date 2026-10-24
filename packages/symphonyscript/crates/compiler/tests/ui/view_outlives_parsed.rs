use symphonyscript::syntax::{NodeView, parse};
use symphonyscript::Source;
use symphonyscript_fixtures::vocabulary;

fn main() {
    let vocab = vocabulary();
    let source = Source {
        path: "main.sym".into(),
        text: "Clip c\n".into(),
    };
    let view: NodeView<'_>;
    {
        let parsed = parse(&source, &vocab);
        view = parsed.root();
    }
    println!("{view}");
}
