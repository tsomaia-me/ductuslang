pub const DEFAULT_TAG: &str = "example";
const SEPARATOR: &str = "----";
const EXPECT: &str = "#expect";

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProbeProgram {
    pub file: String,
    pub line: usize,
    pub tag: String,
    pub text: String,
    pub expect: Option<String>,
}

pub fn probe_programs(file: &str, content: &str) -> Vec<ProbeProgram> {
    let mut programs = Vec::new();
    let mut lines: Vec<(usize, &str)> = Vec::new();
    for (index, line) in content.lines().enumerate() {
        if line.trim_end() == SEPARATOR {
            programs.extend(probe_program(file, &lines));
            lines.clear();
        } else {
            lines.push((index + 1, line));
        }
    }
    programs.extend(probe_program(file, &lines));
    programs
}

fn probe_program(file: &str, lines: &[(usize, &str)]) -> Option<ProbeProgram> {
    let first = lines.iter().position(|(_, line)| !line.trim().is_empty())?;
    let mut body = &lines[first..];
    let mut tag = DEFAULT_TAG.to_string();
    if let Some((_, line)) = body.first()
        && let Some(named) = line.trim().strip_prefix('#')
    {
        tag = named.trim().to_string();
        body = &body[1..];
    }
    let mut expect = None;
    if let Some(last) = body.iter().rposition(|(_, line)| !line.trim().is_empty())
        && let Some(stated) = body[last].1.trim().strip_prefix(EXPECT)
        && (stated.is_empty() || stated.starts_with(char::is_whitespace))
    {
        expect = Some(stated.trim().to_string());
        body = &body[..last];
    }
    let line = body.first().map_or(lines[first].0, |(number, _)| *number);
    let text: String = body.iter().flat_map(|(_, line)| [*line, "\n"]).collect();
    Some(ProbeProgram {
        file: file.to_string(),
        line,
        tag,
        text,
        expect,
    })
}
