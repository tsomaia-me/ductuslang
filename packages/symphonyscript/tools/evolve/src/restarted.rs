use std::fs::{File, OpenOptions};
use std::os::unix::process::CommandExt;
use std::process::Command;

pub const RESUME_AT: &str = "--resume-at";

pub fn resumed_output(path: &str, resume_at: usize) -> std::io::Result<File> {
    if resume_at == 0 {
        return File::create(path);
    }
    OpenOptions::new().append(true).open(path)
}

pub fn restarted(line: usize) -> String {
    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(error) => return format!("could not restart at line {line}: {error}"),
    };
    let mut kept = Vec::new();
    let mut raw = std::env::args().skip(1);
    while let Some(argument) = raw.next() {
        if argument == RESUME_AT {
            raw.next();
        } else {
            kept.push(argument);
        }
    }
    let error = Command::new(executable)
        .args(kept)
        .args([RESUME_AT.to_string(), line.to_string()])
        .exec();
    format!("could not restart at line {line}: {error}")
}
