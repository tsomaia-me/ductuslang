use crate::spec_example::SpecExample;

pub fn document_examples(document: &str) -> Vec<SpecExample> {
    let mut examples: Vec<SpecExample> = Vec::new();
    let mut section = String::new();
    let mut open: Option<SpecExample> = None;
    for (index, line) in document.lines().enumerate() {
        if let Some(example) = open.as_mut() {
            if line == "```" {
                if let Some(done) = open.take() {
                    examples.push(done);
                }
            } else {
                example.text.push_str(line);
                example.text.push('\n');
            }
            continue;
        }
        if let Some(heading) = line
            .strip_prefix("## ")
            .or_else(|| line.strip_prefix("### "))
        {
            section = heading
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .trim_end_matches('.')
                .to_string();
        } else if let Some(tag) = line.strip_prefix("```") {
            let ordinal = 1 + examples
                .iter()
                .filter(|example| example.section == section && example.tag == tag)
                .count();
            open = Some(SpecExample {
                section: section.clone(),
                tag: tag.to_string(),
                ordinal,
                line: index + 2,
                text: String::new(),
            });
        }
    }
    examples
}
