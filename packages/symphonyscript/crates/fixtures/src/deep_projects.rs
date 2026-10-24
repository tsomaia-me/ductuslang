use crate::project::Project;
use crate::project_file::ProjectFile;

const DEPTH: usize = 100_000;
const BODY_DEPTH: usize = 3_000;

fn nested(open: &str, middle: &str, close: &str) -> String {
    format!("{}{middle}{}", open.repeat(DEPTH), close.repeat(DEPTH))
}

fn project(name: &str, text: String) -> Project {
    Project {
        name: name.to_string(),
        files: vec![ProjectFile {
            path: format!("{}.sym", name.replace(' ', "_")),
            text,
        }],
    }
}

pub fn deep_projects() -> Vec<Project> {
    let expressions = format!(
        "input x = true\nfn f(v): v\nlet a = {}\nlet b = {}\nlet c = {}\nlet d = {}x\nlet e = {}\nlet g = {}1\nClip top:\n    Note/{} when d\n    Note/(f(a) + c + g + b{})\n",
        nested("(", "1", ")"),
        nested("[", "1", "]"),
        nested("f(", "1", ")"),
        "not ".repeat(DEPTH),
        nested("\"{", "1", "}\""),
        "-".repeat(DEPTH),
        nested("(", "60", ")"),
        "[0]".repeat(DEPTH),
    );
    let cycle = ["for v{level} in 0..1:", "when x:", "when:"];
    let mut bodies = String::from("input x = true\nClip top:\n");
    let mut indent = 1;
    for level in 0..BODY_DEPTH {
        let pad = " ".repeat(indent);
        let line = cycle[level % cycle.len()].replace("{level}", &level.to_string());
        if line == "when:" {
            bodies.push_str(&format!("{pad}when:\n{pad} x:\n"));
            indent += 1;
        } else {
            bodies.push_str(&format!("{pad}{line}\n"));
        }
        indent += 1;
    }
    bodies.push_str(&format!("{}Note\n", " ".repeat(indent)));
    let array_type = format!(
        "input k: i32{} = {}\nClip c:\n    Note/(k{})\n",
        "[1]".repeat(DEPTH),
        nested("[", "1", "]"),
        "[0]".repeat(DEPTH),
    );
    let mut calls = String::new();
    for index in 0..10_000 {
        calls.push_str(&format!("fn step{index}(x): step{}(x)\n", index + 1));
    }
    calls.push_str(
        "fn step10000(x): x\ninput i = 1\nClip c:\n    Note/(step0(i))\n    Note/(step0(64))\n",
    );
    vec![
        project("deep expressions", expressions),
        project("deep bodies", bodies),
        project("deep array type", array_type),
        project("long call chain", calls),
    ]
}
