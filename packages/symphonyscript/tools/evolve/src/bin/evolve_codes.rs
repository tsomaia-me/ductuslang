#![forbid(unsafe_code)]

use symphonyscript::{Code, Severity};

fn main() {
    for code in Code::ALL {
        let line = serde_json::json!({
            "code": code.as_str(),
            "section": code.section(),
            "warning": code.severity() == Severity::Warning,
        });
        println!("{line}");
    }
}
