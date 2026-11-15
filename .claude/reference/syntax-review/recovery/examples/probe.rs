//! Reviewer's probe for the lexer and parser on broken source.
//! Subcommands:
//!   corpus <file.bin>              property 1, 2, 6 on every source of a length-prefixed corpus
//!   valid <file.bin> <out.bin>     write the sources that parse with no diagnostic
//!   mutate <valid.bin> <max>       property 2 and 6 on mutants of the first <max> valid programs
//!   single <valid.bin> <max> <out> property 4: one fault each, counts diagnostics
//!   multi <valid.bin> <max> <out>  property 5: two and three faults
//!   depth <case> <n> <broken>      property 3: one case on a 1 MiB thread
//!   time <shape> <n> <reps>        property 7: min time and allocation of parse
//!   gen <shape> <n>                print a generated hostile source
//!   show <file>                    print tokens, tree and diagnostics of a source file
use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use symphonyscript::Source;
use symphonyscript::syntax::{self, Parsed, SyntaxKind};
use symphonyscript_core::Vocabulary;

// ---------- allocation counter ----------
struct Counting;
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static COUNT: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
            COUNT.fetch_add(1, Ordering::Relaxed);
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            COUNT.fetch_add(1, Ordering::Relaxed);
            if new_size > layout.size() {
                let d = new_size - layout.size();
                ALLOCATED.fetch_add(d, Ordering::Relaxed);
                let live = LIVE.fetch_add(d, Ordering::Relaxed) + d;
                PEAK.fetch_max(live, Ordering::Relaxed);
            } else {
                LIVE.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        p
    }
}
#[global_allocator]
static GLOBAL: Counting = Counting;

// ---------- io ----------
fn load(path: &str) -> Vec<String> {
    let bytes = std::fs::read(path).expect("read corpus");
    let mut out = Vec::new();
    let mut at = 0;
    while at + 4 <= bytes.len() {
        let len = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        at += 4;
        out.push(String::from_utf8(bytes[at..at + len].to_vec()).expect("utf8"));
        at += len;
    }
    out
}
fn save(path: &str, texts: &[String]) {
    let mut f = std::fs::File::create(path).unwrap();
    for t in texts {
        f.write_all(&(t.len() as u32).to_le_bytes()).unwrap();
        f.write_all(t.as_bytes()).unwrap();
    }
}

fn source(text: &str) -> Source {
    Source {
        path: "main.sym".into(),
        text: text.into(),
    }
}

// ---------- checks ----------
#[derive(Default, Clone, Debug)]
struct Outcome {
    diags: Vec<(String, u32, u32, String)>,
    lex_diags: usize,
    error_tokens: usize,
    error_nodes: Vec<(u32, u32)>,
    detached: usize,
    at_layout: usize,
    whole: Option<String>,
    nest: Option<String>,
    spans: Option<String>,
}

fn whole_check(p: &Parsed, text: &str) -> Option<String> {
    let toks = p.probe_tokens();
    if p.probe_starts_len() != toks.len() + 1 {
        return Some(format!(
            "starts {} != tokens+1 {}",
            p.probe_starts_len(),
            toks.len() + 1
        ));
    }
    let mut at = 0u32;
    let mut joined = String::new();
    for (i, (k, s, e)) in toks.iter().enumerate() {
        if *s != at {
            return Some(format!("token {i} {k:?} starts at {s}, expected {at}"));
        }
        if e < s {
            return Some(format!("token {i} {k:?} ends {e} before start {s}"));
        }
        match text.get(*s as usize..*e as usize) {
            Some(t) => joined.push_str(t),
            None => return Some(format!("token {i} {k:?} range {s}..{e} not on char boundary")),
        }
        at = *e;
    }
    if at as usize != text.len() {
        return Some(format!("tokens end at {at}, text len {}", text.len()));
    }
    if joined != text {
        return Some("joined tokens differ from source".into());
    }
    None
}

fn nest_check(p: &Parsed) -> Option<String> {
    let nodes = p.probe_nodes();
    let ntok = p.probe_tokens().len() as u32;
    if nodes.is_empty() {
        return Some("no nodes".into());
    }
    let mut stack: Vec<u32> = Vec::new();
    for (i, (kind, first, end, fd)) in nodes.iter().enumerate() {
        let i = i as u32;
        if first > end || *end > ntok {
            return Some(format!("node {i} {kind:?} bad tokens {first}..{end} of {ntok}"));
        }
        if *fd > i {
            return Some(format!("node {i} first_desc {fd} > index"));
        }
        // pop children
        let mut children = Vec::new();
        while let Some(&top) = stack.last() {
            if top >= *fd {
                children.push(top);
                stack.pop();
            } else {
                break;
            }
        }
        children.reverse();
        let mut cursor = *first;
        let mut expect_fd = *fd;
        for c in &children {
            let (ck, cf, ce, cfd) = nodes[*c as usize];
            if cfd != expect_fd {
                return Some(format!(
                    "node {i} {kind:?}: child {c} {ck:?} first_desc {cfd} != {expect_fd}"
                ));
            }
            expect_fd = c + 1;
            if cf < cursor || ce > *end || cf > ce {
                return Some(format!(
                    "node {i} {kind:?} {first}..{end}: child {c} {ck:?} {cf}..{ce} out of order or outside (cursor {cursor})"
                ));
            }
            cursor = ce;
        }
        if expect_fd != i {
            return Some(format!("node {i} {kind:?}: descendants do not reach index"));
        }
        stack.push(i);
    }
    let root = nodes.len() as u32 - 1;
    let (rk, rf, re, rfd) = nodes[root as usize];
    if stack.len() != 1 || rk != SyntaxKind::File || rf != 0 || re != ntok || rfd != 0 {
        return Some(format!(
            "root {rk:?} {rf}..{re} fd {rfd}, stack {} (ntok {ntok})",
            stack.len()
        ));
    }
    None
}

fn analyse(text: &str, vocab: &Vocabulary, with_lex: bool) -> Outcome {
    let src = source(text);
    let p = syntax::parse(&src, vocab);
    let mut o = Outcome::default();
    for d in &p.diagnostics {
        o.diags.push((
            format!("{:?}", d.code),
            d.span.range.start,
            d.span.range.end,
            d.summary.clone(),
        ));
    }
    o.error_tokens = p
        .probe_tokens()
        .iter()
        .filter(|t| t.0 == SyntaxKind::Error)
        .count();
    let toks = p.probe_tokens();
    let byte = |t: u32| -> u32 {
        toks.get(t as usize)
            .map(|x| x.1)
            .unwrap_or(text.len() as u32)
    };
    o.error_nodes = p
        .probe_nodes()
        .iter()
        .filter(|n| n.0 == SyntaxKind::ErrorNode)
        .map(|n| (byte(n.1), byte(n.2)))
        .collect();
    o.detached = p.probe_detached().len();
    o.at_layout = p.probe_at_layout().len();
    o.whole = whole_check(&p, text);
    o.nest = nest_check(&p);
    for d in &p.diagnostics {
        let r = &d.span.range;
        if r.start > r.end
            || r.end as usize > text.len()
            || !text.is_char_boundary(r.start as usize)
            || !text.is_char_boundary(r.end as usize)
        {
            o.spans = Some(format!("diagnostic span {r:?} bad (len {})", text.len()));
        }
    }
    if with_lex {
        o.lex_diags = syntax::lex(&src, vocab).diagnostics.len();
    }
    o
}

/// Runs `f` on each item in a worker thread with a watchdog; reports panics and hangs.
fn run_guarded<T: Send + 'static, R: Send + 'static>(
    items: Vec<T>,
    stack: usize,
    f: impl Fn(&Vocabulary, &T) -> R + Send + 'static,
    mut on_result: impl FnMut(usize, Result<R, String>, Duration),
) {
    let n = items.len();
    let (tx, rx) = mpsc::channel::<(usize, Result<R, String>, Duration)>();
    let (stx, srx) = mpsc::channel::<usize>();
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || {
            let vocab = symphonyscript_fixtures::vocabulary();
            for (i, it) in items.iter().enumerate() {
                stx.send(i).ok();
                let t0 = Instant::now();
                let r = panic::catch_unwind(AssertUnwindSafe(|| f(&vocab, it)));
                let dt = t0.elapsed();
                let r = r.map_err(|e| {
                    e.downcast_ref::<String>()
                        .cloned()
                        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "panic".into())
                });
                tx.send((i, r, dt)).ok();
            }
        })
        .unwrap();
    let mut done = 0;
    let mut current = 0usize;
    while done < n {
        while let Ok(i) = srx.try_recv() {
            current = i;
        }
        match rx.recv_timeout(Duration::from_secs(60)) {
            Ok((i, r, dt)) => {
                on_result(i, r, dt);
                done += 1;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                while let Ok(i) = srx.try_recv() {
                    current = i;
                }
                eprintln!("HANG: item {current} exceeded 60 s");
                std::process::exit(3);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!("worker died at item {current}");
                std::process::exit(4);
            }
        }
    }
}

struct Tally {
    name: String,
    total: usize,
    panics: Vec<(usize, String)>,
    not_whole: Vec<(usize, String)>,
    not_nested: Vec<(usize, String)>,
    bad_spans: Vec<(usize, String)>,
    clean_with_recovery: Vec<(usize, String)>,
    recovery_without_diag: Vec<(usize, String)>,
    slowest: (Duration, usize),
}
impl Tally {
    fn new(name: &str) -> Self {
        Tally {
            name: name.into(),
            total: 0,
            panics: vec![],
            not_whole: vec![],
            not_nested: vec![],
            bad_spans: vec![],
            clean_with_recovery: vec![],
            recovery_without_diag: vec![],
            slowest: (Duration::ZERO, 0),
        }
    }
    fn add(&mut self, i: usize, r: Result<Outcome, String>, dt: Duration) {
        self.total += 1;
        if dt > self.slowest.0 {
            self.slowest = (dt, i);
        }
        let o = match r {
            Ok(o) => o,
            Err(e) => {
                self.panics.push((i, e));
                return;
            }
        };
        if let Some(w) = o.whole.clone() {
            self.not_whole.push((i, w));
        }
        if let Some(w) = o.nest.clone() {
            self.not_nested.push((i, w));
        }
        if let Some(w) = o.spans.clone() {
            self.bad_spans.push((i, w));
        }
        let recovery = o.error_tokens > 0 || !o.error_nodes.is_empty() || o.detached > 0 || o.at_layout > 0;
        if o.diags.is_empty() && recovery {
            self.clean_with_recovery.push((
                i,
                format!(
                    "errtok {} errnodes {} detached {} at_layout {}",
                    o.error_tokens,
                    o.error_nodes.len(),
                    o.detached,
                    o.at_layout
                ),
            ));
        }
        if o.diags.is_empty() && (o.error_tokens > 0 || !o.error_nodes.is_empty()) {
            self.recovery_without_diag.push((i, String::new()));
        }
    }
    fn print(&self, texts: &[String]) {
        println!(
            "[{}] total {} panics {} not_whole {} not_nested {} bad_spans {} clean_with_recovery {} slowest {:?} (item {})",
            self.name,
            self.total,
            self.panics.len(),
            self.not_whole.len(),
            self.not_nested.len(),
            self.bad_spans.len(),
            self.clean_with_recovery.len(),
            self.slowest.0,
            self.slowest.1
        );
        let show = |label: &str, v: &Vec<(usize, String)>| {
            let mut v = v.clone();
            v.sort_by_key(|(i, _)| texts.get(*i).map_or(0, |t| t.len()));
            for (i, m) in v.iter().take(4) {
                println!("  {label} item {i}: {m}");
                if let Some(t) = texts.get(*i) {
                    println!("    src {:?}", truncate(t, 400));
                }
            }
        };
        show("PANIC", &self.panics);
        show("NOT_WHOLE", &self.not_whole);
        show("NOT_NESTED", &self.not_nested);
        show("BAD_SPAN", &self.bad_spans);
        show("CLEAN_WITH_RECOVERY", &self.clean_with_recovery);
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        return s.to_string();
    }
    let mut end = n;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…(+{} bytes)", &s[..end], s.len() - end)
}

// ---------- token helpers for mutation ----------
#[derive(Clone, Debug)]
struct Tok {
    kind: SyntaxKind,
    start: usize,
    end: usize,
}
fn tokens_of(text: &str, vocab: &Vocabulary) -> Vec<Tok> {
    let p = syntax::parse(&source(text), vocab);
    p.probe_tokens()
        .into_iter()
        .map(|(k, s, e)| Tok {
            kind: k,
            start: s as usize,
            end: e as usize,
        })
        .collect()
}
fn is_sig(k: SyntaxKind) -> bool {
    !matches!(
        k,
        SyntaxKind::Whitespace
            | SyntaxKind::Comment
            | SyntaxKind::Newline
            | SyntaxKind::Indent
            | SyntaxKind::Dedent
    )
}
fn word_char(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '#')
}
fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind('\n').map_or(0, |i| i + 1)
}
fn first_on_line(text: &str, at: usize) -> bool {
    text[line_start(text, at)..at].chars().all(|c| c == ' ')
}

#[derive(Clone, Debug)]
struct Edit {
    at: usize,
    del: usize,
    ins: String,
    class: &'static str,
}
fn apply(text: &str, edits: &[Edit]) -> String {
    let mut e: Vec<&Edit> = edits.iter().collect();
    e.sort_by_key(|x| std::cmp::Reverse(x.at));
    let mut s = text.to_string();
    for x in e {
        s.replace_range(x.at..x.at + x.del, &x.ins);
    }
    s
}

/// Deletes one token without merging its neighbours or shifting the next token's column.
fn delete_token(text: &str, t: &Tok) -> Edit {
    let before = text[..t.start].chars().next_back();
    let after = text[t.end..].chars().next();
    if first_on_line(text, t.start) {
        // delete the token and the spaces after it, then pad so the next token keeps... no:
        // keep the line's indentation; the next token moves to the token's column.
        let rest = &text[t.end..];
        let spaces = rest.len() - rest.trim_start_matches(' ').len();
        return Edit { at: t.start, del: t.end - t.start + spaces, ins: String::new(), class: "del" };
    }
    let ins = if word_char(before) && word_char(after) { " " } else { "" };
    Edit { at: t.start, del: t.end - t.start, ins: ins.into(), class: "del" }
}

/// Single-fault edits for one valid program.
fn single_faults(text: &str, toks: &[Tok]) -> Vec<Edit> {
    let mut out = Vec::new();
    let sig: Vec<&Tok> = toks.iter().filter(|t| is_sig(t.kind) && t.end > t.start).collect();
    for t in &sig {
        let mut e = delete_token(text, t);
        e.class = "delete-token";
        out.push(e);
        if matches!(t.kind, SyntaxKind::RParen | SyntaxKind::RBracket | SyntaxKind::RBrace) {
            let mut e = delete_token(text, t);
            e.class = "missing-closer";
            out.push(e);
        }
        if t.kind == SyntaxKind::StringEnd {
            out.push(Edit { at: t.start, del: 1, ins: String::new(), class: "open-string" });
        }
    }
    // stray tokens before a significant token
    let strays = [")", "]", "}", ",", "=", ":", "+", "|", "foo", "let", "1", "(", "["];
    for (n, t) in sig.iter().enumerate() {
        let stray = strays[n % strays.len()];
        let ins = if first_on_line(text, t.start) {
            format!("{stray} ")
        } else {
            let before = text[..t.start].chars().next_back();
            if before == Some(' ') { format!("{stray} ") } else { format!(" {stray} ") }
        };
        out.push(Edit { at: t.start, del: 0, ins, class: "stray-token" });
    }
    // per line: mis-indent and tab
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches('\n').trim_end_matches('\r');
        let indent = body.len() - body.trim_start_matches(' ').len();
        let blank = body.trim().is_empty() || body.trim_start().starts_with("//");
        if !blank {
            out.push(Edit { at, del: 0, ins: " ".into(), class: "indent-plus-one" });
            if indent > 0 {
                out.push(Edit { at, del: 1, ins: String::new(), class: "indent-minus-one" });
                out.push(Edit { at, del: 1, ins: "\t".into(), class: "tab-for-space" });
            }
        }
        at += line.len();
    }
    out
}

// ---------- mutants for property 2 ----------
fn mutants(text: &str, toks: &[Tok], seed: &mut u64) -> Vec<String> {
    let mut out = Vec::new();
    // truncation at every char boundary
    for (i, _) in text.char_indices() {
        out.push(text[..i].to_string());
    }
    let sig: Vec<&Tok> = toks.iter().filter(|t| is_sig(t.kind) && t.end > t.start).collect();
    for (n, t) in sig.iter().enumerate() {
        // delete raw
        let mut s = text.to_string();
        s.replace_range(t.start..t.end, "");
        out.push(s);
        // double
        let mut s = text.to_string();
        s.insert_str(t.start, &text[t.start..t.end]);
        out.push(s);
        // swap with next
        if let Some(u) = sig.get(n + 1) {
            let mut s = String::new();
            s.push_str(&text[..t.start]);
            s.push_str(&text[u.start..u.end]);
            s.push_str(&text[t.end..u.start]);
            s.push_str(&text[t.start..t.end]);
            s.push_str(&text[u.end..]);
            out.push(s);
        }
    }
    let rnd = |seed: &mut u64| {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        *seed
    };
    let pick_at = |seed: &mut u64| -> usize {
        let b: Vec<usize> = text.char_indices().map(|(i, _)| i).chain([text.len()]).collect();
        b[(rnd(seed) as usize) % b.len()]
    };
    let inserts = [
        ")", "]", "}", "(", "[", "{", "\"", "\t", "\r", "\0", "\u{1}", "\u{7f}", "\u{feff}", "\u{200b}",
        "    ", "\n    ", "\n\t", "\r\n", "é", "名前", "🎵", "#", "_", "^", ";", "==", "\\", "{\"",
        "json {", "json [", "\"{", "//", "|", ":", "::", "..", "'", "\u{301}",
    ];
    for ins in inserts {
        for _ in 0..6 {
            let at = pick_at(seed);
            let mut s = text.to_string();
            s.insert_str(at, ins);
            out.push(s);
        }
    }
    // CR-only line ends
    out.push(text.replace('\n', "\r"));
    out.push(text.replace('\n', "\r\n"));
    out.push(text.replace("    ", "\t"));
    out.push(text.replace("    ", "  "));
    // non-ASCII identifiers: replace ASCII letters in identifiers
    out.push(text.replace('a', "á"));
    out.push(text.replace('e', "名"));
    // long line: duplicate one line's content many times
    if let Some(line) = text.lines().find(|l| !l.trim().is_empty()) {
        let long = format!("{}\n", line.repeat(2000));
        out.push(format!("{long}{text}"));
    }
    out
}

// ---------- hostile shapes ----------
fn hostile(shape: &str, n: usize) -> String {
    let mut s = String::new();
    match shape {
        "unclosed-paren" => {
            s.push_str("let x = ");
            s.push_str(&"(".repeat(n));
            s.push_str("1\n");
        }
        "unclosed-bracket" => {
            s.push_str("let x = ");
            s.push_str(&"[".repeat(n));
            s.push_str("1\n");
        }
        "unclosed-brace" => {
            s.push_str("let x = ");
            s.push_str(&"{1: ".repeat(n));
            s.push_str("1\n");
        }
        "unclosed-lines" => {
            // N lines each opening a bracket that never closes
            for i in 0..n {
                s.push_str(&format!("let x{i} = (1 +\n"));
            }
        }
        "stray-closers" => {
            s.push_str("let x = 1");
            s.push_str(&")".repeat(n));
            s.push('\n');
        }
        "stray-closer-lines" => {
            for i in 0..n {
                s.push_str(&format!("let x{i} = 1)\n"));
            }
        }
        "faulty-lines" => {
            for i in 0..n {
                s.push_str(&format!("let x{i} = = 1\n"));
            }
        }
        "faulty-lines-in-bracket" => {
            s.push_str("let x = (\n");
            for _ in 0..n {
                s.push_str("    1 + + 2\n");
            }
        }
        "faulty-lines-in-bracket-closed" => {
            s.push_str("let x = foo(\n");
            for _ in 0..n {
                s.push_str("    1 + + 2,\n");
            }
            s.push_str(")\n");
        }
        "fault-then-valid" => {
            s.push_str("let x = = 1\n");
            for i in 0..n {
                s.push_str(&format!("let y{i} = {i}\n"));
            }
        }
        "open-bracket-then-valid" => {
            s.push_str("let x = (1 +\n");
            for i in 0..n {
                s.push_str(&format!("let y{i} = {i}\n"));
            }
        }
        "open-bracket-then-placements" => {
            s.push_str("let x = (1 +\n");
            for i in 0..n {
                s.push_str(&format!("Clip c{i}:\n    Note/{}\n", i % 100));
            }
        }
        "open-strings" => {
            for i in 0..n {
                s.push_str(&format!("let x{i} = \"abc\n"));
            }
        }
        "open-strings-one-line" => {
            s.push_str("let x = ");
            for _ in 0..n {
                s.push_str("\"{");
            }
            s.push('\n');
        }
        "misindented" => {
            s.push_str("Clip c:\n");
            for i in 0..n {
                let pad = if i % 2 == 0 { "    " } else { "  " };
                s.push_str(&format!("{pad}Note/{}\n", i % 100));
            }
        }
        "misindented-unexpected" => {
            for i in 0..n {
                s.push_str(&format!("let x{i} = 1\n  let z{i} = 2\n"));
            }
        }
        "long-line" => {
            s.push_str("let x = 1");
            for _ in 0..n {
                s.push_str(" + 1");
            }
            s.push('\n');
        }
        "long-line-faulty" => {
            s.push_str("let x = 1");
            for _ in 0..n {
                s.push_str(" + + 1");
            }
            s.push('\n');
        }
        "long-line-in-bracket-faulty" => {
            s.push_str("let x = (1");
            for _ in 0..n {
                s.push_str(" 1 1");
            }
            s.push('\n');
        }
        "long-placement-line" => {
            s.push_str("Scene s:\n    ");
            for _ in 0..n {
                s.push_str("Cue/1 ");
            }
            s.push('\n');
        }
        "attr-clause" => {
            s.push_str("Scene s:\n    Cue/1 | at=1\n");
            for _ in 0..n {
                s.push_str("            fade=2\n");
            }
        }
        "attr-clause-faulty" => {
            s.push_str("Scene s:\n    Cue/1 | at=1\n");
            for _ in 0..n {
                s.push_str("            fade= =2\n");
            }
        }
        "nested-blocks-broken" => {
            // N nested blocks each with a fault on its header line
            s.push_str("Rig r:\n");
            for i in 0..n {
                let pad = " ".repeat(4 + (i % 50) * 2);
                s.push_str(&format!("{pad}when = x:\n"));
            }
            s.push_str("    Lamp/1\n");
        }
        "nested-blocks-broken2" => {
            // depth grows: each block header broken, body one level deeper
            for i in 0..n {
                let pad = " ".repeat(i);
                s.push_str(&format!("{pad}Rig r{i} = :\n"));
            }
        }
        "fn-bodies-broken" => {
            for i in 0..n {
                s.push_str(&format!("fn f{i}(x):\n    let a = = x\n    a\n"));
            }
        }
        "deep-indent-valid" => {
            // depth n nested blocks (when blocks), indentation 1 more each
            s.push_str("Rig r:\n");
            for i in 0..n {
                let pad = " ".repeat(1 + i);
                s.push_str(&format!("{pad}when x:\n"));
            }
            s.push_str(&" ".repeat(n + 1));
            s.push_str("Lamp/1\n");
        }
        "many-dedents-mismatch" => {
            s.push_str("Rig r:\n");
            for i in 0..n {
                let pad = " ".repeat(1 + i);
                s.push_str(&format!("{pad}when x:\n"));
            }
            s.push_str(&" ".repeat(n + 1));
            s.push_str("Lamp/1\n");
            // land between columns
            s.push_str("Rig q:\n");
        }
        "unclosed-bundle" => {
            s.push_str("Scene s:\n");
            for _ in 0..n {
                s.push_str("    [Cue/1 Cue/2\n");
            }
        }
        "unclosed-paren-placements" => {
            s.push_str("Scene s:\n");
            for _ in 0..n {
                s.push_str("    (Cue/1\n");
            }
        }
        "unclosed-call-lines" => {
            s.push_str("Scene s:\n");
            for _ in 0..n {
                s.push_str("    Cue/(f(1\n");
            }
        }
        "dedent-ladder" => {
            for i in 0..n {
                s.push_str(&format!("Rig r{i}:\n    Lamp/1\n   Lamp/2\n"));
            }
        }
        "tabs" => {
            s.push_str("Rig r:\n");
            for _ in 0..n {
                s.push_str("\tLamp/1\n");
            }
        }
        "semicolons" => {
            for i in 0..n {
                s.push_str(&format!("let x{i} = 1;\n"));
            }
        }
        "json-unclosed" => {
            s.push_str("let x = json ");
            for _ in 0..n {
                s.push_str("{\"a\": ");
            }
            s.push('\n');
        }
        "interp-unclosed-lines" => {
            for i in 0..n {
                s.push_str(&format!("let x{i} = \"a{{b\n"));
            }
        }
        "unclosed-type-args" => {
            for i in 0..n {
                s.push_str(&format!("let x{i}: Foo[i32 = 1\n"));
            }
        }
        "match-arms-broken" => {
            s.push_str("let x = match y:\n");
            for _ in 0..n {
                s.push_str("    1 2: 3\n");
            }
        }
        "open-lines-mixed" => {
            s.push_str("let y = ");
            for _ in 0..n {
                s.push_str("([{\n");
            }
            s.push_str("1\n");
        }
        "open-lines-paren" => {
            s.push_str("let y = ");
            for _ in 0..n {
                s.push_str("(\n");
            }
            s.push_str("1\n");
        }
        "open-lines-bracket" => {
            s.push_str("let y = ");
            for _ in 0..n {
                s.push_str("[\n");
            }
            s.push_str("1\n");
        }
        "open-lines-brace" => {
            s.push_str("let y = ");
            for _ in 0..n {
                s.push_str("{\n");
            }
            s.push_str("1\n");
        }
        "open-lines-brace-entry" => {
            s.push_str("let y = ");
            for _ in 0..n {
                s.push_str("{1:\n");
            }
            s.push_str("1\n");
        }
        "unclosed-one-line-mixed" => {
            s.push_str("let y = ");
            for _ in 0..n {
                s.push_str("([{");
            }
            s.push_str("1\n");
        }
        "faulty-lines-in-call" => {
            s.push_str("let x = f(\n");
            for _ in 0..n {
                s.push_str("    1 1,\n");
            }
        }
        "faulty-lines-in-bracket-col0" => {
            s.push_str("let x = (\n");
            for _ in 0..n {
                s.push_str("1 + + 2\n");
            }
        }
        "stray-closer-lines-nested" => {
            s.push_str("Rig r:\n");
            for _ in 0..n {
                s.push_str("    Lamp/1)\n");
            }
        }
        "placement-lines-open-group" => {
            for i in 0..n {
                s.push_str(&format!("Clip c{i} | gain=f((1\n"));
            }
        }
        "open-group-then-placements" => {
            s.push_str("Clip a | gain=f((1\n");
            for i in 0..n {
                s.push_str(&format!("Clip c{i}:\n    Note/{}\n", i % 100));
            }
        }
        "placement-lines-open-call" => {
            for i in 0..n {
                s.push_str(&format!("Clip c{i} | gain=f(1\n"));
            }
        }
        "body-lines-open-group" => {
            s.push_str("Scene s:\n");
            for _ in 0..n {
                s.push_str("    Cue/(f((1\n");
            }
        }
        "placement-lines-open-type" => {
            for i in 0..n {
                s.push_str(&format!("Clip c{i} | gain=Foo[(1\n"));
            }
        }
        "placement-lines-open-brace" => {
            for i in 0..n {
                s.push_str(&format!("Clip c{i} | gain={{1: (\n"));
            }
        }
        "nested-map-lines-valid" => {
            s.push_str("let y = ");
            for _ in 0..n {
                s.push_str("{1:\n");
            }
            s.push('1');
            s.push_str(&"}".repeat(n));
            s.push('\n');
        }
        "nested-json-lines-valid" => {
            s.push_str("let y = json ");
            for _ in 0..n {
                s.push_str("{\"a\":\n");
            }
            s.push('1');
            s.push_str(&"}".repeat(n));
            s.push('\n');
        }
        "nested-map-one-line-valid" => {
            s.push_str("let y = ");
            for _ in 0..n {
                s.push_str("{1: ");
            }
            s.push('1');
            s.push_str(&"}".repeat(n));
            s.push('\n');
        }
        _ => panic!("unknown shape {shape}"),
    }
    s
}

// ---------- depth cases ----------
fn depth_source(case: &str, n: usize, broken: bool) -> String {
    let rep = |a: &str, k: usize| a.repeat(k);
    let (open, mid, close): (String, &str, String) = match case {
        "paren" => (rep("(", n), "1", rep(")", n)),
        "bracket" => (rep("[", n), "1", rep("]", n)),
        "map" => (rep("{1: ", n), "1", rep("}", n)),
        "record" => (rep("Hit { note: ", n), "1", rep(" }", n)),
        "call" => (rep("f(", n), "1", rep(")", n)),
        "index" => (String::from("a"), "", rep("[0]", n)),
        "interp" => (rep("\"{", n), "1", rep("}\"", n)),
        "json-array" => (format!("json {}", rep("[", n)), "1", rep("]", n)),
        "json-object" => (format!("json {}", rep("{\"a\": ", n)), "1", rep("}", n)),
        "unary" => (rep("-", n), "1", String::new()),
        "unary-spaced" => (rep("- ", n), "1", String::new()),
        "not" => (rep("not ", n), "x", String::new()),
        "pow" => (rep("2 ** ", n), "2", String::new()),
        "fallback" => (rep("a ?? ", n), "1", String::new()),
        "add" => (rep("1 + ", n), "1", String::new()),
        "if" => (rep("if a then ", n), "1", rep(" else 1", n)),
        "lambda" => (String::from("f("), "", String::new()),
        "dot" => (String::from("a"), "", rep(".b", n)),
        "conv" => (rep("f32(", n), "1", rep(")", n)),
        "for-expr" => (rep("for i in 0..1 into a from 0: ", n), "1", String::new()),
        "type-paren" => (String::new(), "", String::new()),
        _ => (String::new(), "", String::new()),
    };
    match case {
        "lambda" => {
            let body = format!("{}{}", rep("x: ", n), "x");
            let tail = if broken { "" } else { ")" };
            return format!("let y = f({body}{tail}\n");
        }
        "type-paren" => {
            let close = if broken { String::new() } else { rep(")", n) };
            return format!("let y: {}i32{} = 1\n", rep("(", n), close);
        }
        "type-args" => {
            let close = if broken { String::new() } else { rep("]", n) };
            return format!("let y: {}i32{} = 1\n", rep("Foo[", n), close);
        }
        "fn-type" => {
            let t = format!("{}i32", rep("() -> ", n));
            let t = if broken { format!("{t} ->") } else { t };
            return format!("let y: {t} = 1\n");
        }
        "sum-type" => {
            let t = format!("i32{}", rep(" | i32", n));
            let t = if broken { format!("{t} |") } else { t };
            return format!("let y: {t} = 1\n");
        }
        "paren-placement" => {
            let close = if broken { String::new() } else { rep(")", n) };
            return format!("Scene s:\n    {}Cue{}\n", rep("(", n), close);
        }
        "bundle" => {
            let close = if broken { String::new() } else { rep("]", n) };
            return format!("Scene s:\n    {}Cue{}\n", rep("[", n), close);
        }
        "default-arg" => {
            let close = if broken { String::new() } else { rep(")", n) };
            return format!("Scene s:\n    Cue/{}1{}\n", rep("(", n), close);
        }
        "indent" => {
            // n nested when blocks, one column deeper each (bytes ~ n^2/2)
            let mut s = String::from("Rig r:\n");
            for i in 0..n {
                s.push_str(&" ".repeat(1 + i));
                s.push_str(if broken && i % 2 == 1 { "when = x:\n" } else { "when x:\n" });
            }
            s.push_str(&" ".repeat(n + 1));
            s.push_str("Lamp/1\n");
            return s;
        }
        "indent-children" => {
            let mut s = String::from("Synth r:\n");
            for i in 0..n {
                s.push_str(&" ".repeat(1 + i));
                s.push_str(if broken && i % 2 == 1 { "Synth = :\n" } else { "Synth:\n" });
            }
            return s;
        }
        "for-block" => {
            let mut s = String::from("Rig r:\n");
            for i in 0..n {
                s.push_str(&" ".repeat(1 + i));
                s.push_str(if broken && i % 2 == 1 { "for i in :\n" } else { "for i in 0..2:\n" });
            }
            s.push_str(&" ".repeat(n + 1));
            s.push_str("Lamp/1\n");
            return s;
        }
        _ => {}
    }
    if open.is_empty() && mid.is_empty() && close.is_empty() {
        panic!("unknown depth case {case}");
    }
    let close = if broken {
        match case {
            "unary" | "unary-spaced" | "not" | "pow" | "fallback" | "add" | "for-expr" => {
                // broken: drop the operand
                String::new()
            }
            _ => String::new(),
        }
    } else {
        close
    };
    let mid = if broken
        && matches!(case, "unary" | "unary-spaced" | "not" | "pow" | "fallback" | "add" | "for-expr")
    {
        ""
    } else {
        mid
    };
    let mid = if broken && case == "if" { "1 else" } else { mid };
    format!("let y = {open}{mid}{close}\n")
}

fn show(text: &str, vocab: &Vocabulary) {
    let p = syntax::parse(&source(text), vocab);
    println!("--- diagnostics ({})", p.diagnostics.len());
    for d in &p.diagnostics {
        let r = &d.span.range;
        println!(
            "  {:?} {:?} {:?} :: {}",
            d.code,
            r,
            text.get(r.start as usize..r.end as usize).unwrap_or("?"),
            d.summary
        );
    }
    println!("--- detached {:?} at_layout {:?}", p.probe_detached(), p.probe_at_layout());
    println!("--- tree");
    let mut depth = 0usize;
    for w in p.root().preorder() {
        match w {
            syntax::Walk::Enter(e) => {
                match e {
                    syntax::ElementView::Node(n) => {
                        println!("{}{:?} {:?}", "  ".repeat(depth), n.kind(), n.range());
                        depth += 1;
                    }
                    syntax::ElementView::Token(t) => {
                        if !matches!(t.kind(), SyntaxKind::Whitespace) {
                            println!("{}{:?} {:?}", "  ".repeat(depth), t.kind(), t.text());
                        }
                    }
                }
            }
            syntax::Walk::Leave(e) => {
                if let syntax::ElementView::Node(_) = e {
                    depth -= 1;
                }
            }
        }
    }
}

fn top_items(text: &str, vocab: &Vocabulary) -> Vec<(u32, u32)> {
    let p = syntax::parse(&source(text), vocab);
    p.root().children().map(|c| (c.range().start, c.range().end)).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("");
    match cmd {
        "corpus" => {
            let texts = load(&args[2]);
            let items = texts.clone();
            let mut tally = Tally::new(&args[2]);
            let mut clean = 0;
            run_guarded(items, 2 << 20, |v, t: &String| analyse(t, v, false), |i, r, dt| {
                if let Ok(o) = &r {
                    if o.diags.is_empty() {
                        clean += 1;
                    }
                }
                tally.add(i, r, dt)
            });
            println!("clean (no diagnostic): {clean}");
            tally.print(&texts);
        }
        "valid" => {
            let texts = load(&args[2]);
            let vocab = symphonyscript_fixtures::vocabulary();
            let mut out: Vec<String> = texts
                .into_iter()
                .filter(|t| syntax::parse(&source(t), &vocab).diagnostics.is_empty())
                .collect();
            // add SPEC sym examples that parse clean
            for ex in symphonyscript_fixtures::spec_examples() {
                if ex.tag == "sym" && syntax::parse(&source(&ex.text), &vocab).diagnostics.is_empty() {
                    out.push(ex.text.clone());
                }
            }
            out.sort();
            out.dedup();
            println!("valid {}", out.len());
            save(&args[3], &out);
        }
        "mutate" => {
            let valid = load(&args[2]);
            let max: usize = args[3].parse().unwrap();
            let maxlen: usize = args.get(4).map_or(600, |a| a.parse().unwrap());
            let vocab = symphonyscript_fixtures::vocabulary();
            let mut seed = 0x9e3779b97f4a7c15u64;
            let mut all = Vec::new();
            let mut programs = 0;
            for t in valid.iter().filter(|t| t.len() <= maxlen && t.len() > 20) {
                if programs >= max {
                    break;
                }
                programs += 1;
                let toks = tokens_of(t, &vocab);
                all.extend(mutants(t, &toks, &mut seed));
            }
            println!("programs {programs} mutants {}", all.len());
            let texts = all.clone();
            let mut tally = Tally::new("mutants");
            run_guarded(all, 2 << 20, |v, t: &String| analyse(t, v, false), |i, r, dt| tally.add(i, r, dt));
            tally.print(&texts);
        }
        "single" | "multi" => {
            let valid = load(&args[2]);
            let max: usize = args[3].parse().unwrap();
            let out_path = &args[4];
            let vocab = symphonyscript_fixtures::vocabulary();
            let mut cases: Vec<(String, String, Vec<Edit>, String)> = Vec::new(); // (orig, mutated, edits, category)
            let mut seed = 0x1234_5678_9abc_def1u64;
            let mut rnd = move || {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                seed
            };
            let mut programs = 0;
            for t in valid.iter().filter(|t| t.len() <= 1500 && t.len() > 20) {
                if programs >= max {
                    break;
                }
                programs += 1;
                let toks = tokens_of(t, &vocab);
                let faults = single_faults(t, &toks);
                if cmd == "single" {
                    for e in faults {
                        let m = apply(t, std::slice::from_ref(&e));
                        cases.push((t.clone(), m, vec![e.clone()], e.class.to_string()));
                    }
                } else {
                    // keep only faults that draw exactly one message alone
                    let mut ones: Vec<(Edit, Outcome)> = Vec::new();
                    for e in faults {
                        let m = apply(t, std::slice::from_ref(&e));
                        let o = analyse(&m, &vocab, false);
                        if o.diags.len() == 1 && o.whole.is_none() {
                            ones.push((e, o));
                        }
                    }
                    if ones.len() < 2 {
                        continue;
                    }
                    let items = top_items(t, &vocab);
                    let item_of = |at: usize| items.iter().position(|(s, e)| (at as u32) >= *s && (at as u32) < *e);
                    let line_of = |at: usize| t[..at.min(t.len())].matches('\n').count();
                    for _ in 0..40 {
                        let k = if rnd() % 3 == 0 { 3 } else { 2 };
                        let mut pick: Vec<usize> = Vec::new();
                        for _ in 0..k {
                            pick.push((rnd() as usize) % ones.len());
                        }
                        pick.sort();
                        pick.dedup();
                        if pick.len() < 2 {
                            continue;
                        }
                        let edits: Vec<Edit> = pick.iter().map(|i| ones[*i].0.clone()).collect();
                        // edits must not overlap or touch
                        let mut sorted = edits.clone();
                        sorted.sort_by_key(|e| e.at);
                        if sorted.windows(2).any(|w| w[0].at + w[0].del >= w[1].at) {
                            continue;
                        }
                        let items_: Vec<Option<usize>> = sorted.iter().map(|e| item_of(e.at)).collect();
                        let lines: Vec<usize> = sorted.iter().map(|e| line_of(e.at)).collect();
                        let category = if items_.iter().all(|x| x.is_some())
                            && {
                                let mut v = items_.clone();
                                v.dedup();
                                v.len() == items_.len()
                            } {
                            "separate-items"
                        } else if lines.windows(2).all(|w| w[0] != w[1]) && items_.windows(2).all(|w| w[0] == w[1]) {
                            "separate-lines-one-item"
                        } else if lines.windows(2).all(|w| w[0] == w[1]) {
                            "one-line"
                        } else {
                            "mixed"
                        };
                        // is a later fault inside an earlier fault's skipped region (alone)?
                        let mut inside = false;
                        for (a, ea) in sorted.iter().enumerate() {
                            let oa = &ones.iter().find(|(e, _)| e.at == ea.at && e.ins == ea.ins && e.del == ea.del).unwrap().1;
                            for eb in sorted.iter().skip(a + 1) {
                                // offsets of eb in ea-alone text
                                let shift = ea.ins.len() as i64 - ea.del as i64;
                                let pos = (eb.at as i64 + shift) as u32;
                                if oa.error_nodes.iter().any(|(s, e)| pos >= *s && pos <= *e) {
                                    inside = true;
                                }
                            }
                        }
                        let cat = format!("{category}{}", if inside { "/inside-skip" } else { "" });
                        let m = apply(t, &edits);
                        cases.push((t.clone(), m, sorted, cat));
                    }
                }
            }
            println!("programs {programs} cases {}", cases.len());
            let items: Vec<String> = cases.iter().map(|c| c.1.clone()).collect();
            let mut results: Vec<Option<Outcome>> = vec![None; items.len()];
            let mut tally = Tally::new(cmd);
            run_guarded(items.clone(), 2 << 20, |v, t: &String| analyse(t, v, true), |i, r, dt| {
                if let Ok(o) = &r {
                    results[i] = Some(o.clone());
                }
                tally.add(i, r, dt)
            });
            tally.print(&items);
            // summarise counts per category
            use std::collections::BTreeMap;
            let mut table: BTreeMap<String, [usize; 4]> = BTreeMap::new();
            let mut f = std::fs::File::create(out_path).unwrap();
            for (i, c) in cases.iter().enumerate() {
                let Some(o) = &results[i] else { continue };
                let want = c.2.len();
                let n = o.diags.len();
                let row = table.entry(c.3.clone()).or_default();
                row[0] += 1;
                if n == want {
                    row[1] += 1;
                } else if n < want {
                    row[2] += 1;
                } else {
                    row[3] += 1;
                }
                if n != want {
                    writeln!(
                        f,
                        "=== case {i} category {} faults {} diags {} lexdiags {} edits {:?}\n--- orig\n{}\n--- mutated\n{}\n--- diags",
                        c.3, want, n, o.lex_diags,
                        c.2.iter().map(|e| (e.class, e.at, e.del, e.ins.clone())).collect::<Vec<_>>(),
                        c.0, c.1
                    )
                    .unwrap();
                    for d in &o.diags {
                        writeln!(f, "  {} {}..{} {:?} :: {}", d.0, d.1, d.2, c.1.get(d.1 as usize..d.2 as usize).unwrap_or("?"), d.3).unwrap();
                    }
                    writeln!(f, "--- error nodes {:?}", o.error_nodes).unwrap();
                }
            }
            println!("category                         total  exact  fewer  more");
            for (k, v) in &table {
                println!("{k:<32} {:>6} {:>6} {:>6} {:>6}", v[0], v[1], v[2], v[3]);
            }
        }
        "depth" => {
            let case = args[2].clone();
            let (n, broken, text) = if case == "file" {
                (0usize, false, std::fs::read_to_string(&args[3]).unwrap())
            } else {
                let n: usize = args[3].parse().unwrap();
                let broken = args.get(4).is_some_and(|a| a == "broken");
                (n, broken, depth_source(&case, n, broken))
            };
            let len = text.len();
            let h = std::thread::Builder::new()
                .stack_size(1 << 20)
                .spawn(move || {
                    let vocab = symphonyscript_fixtures::vocabulary();
                    let t0 = Instant::now();
                    let p = syntax::parse(&source(&text), &vocab);
                    let dt = t0.elapsed();
                    let whole = whole_check(&p, &text);
                    let nest = nest_check(&p);
                    let nd = p.diagnostics.len();
                    let first = p.diagnostics.first().map(|d| format!("{:?} {:?} {}", d.code, d.span.range, d.summary));
                    drop(p);
                    (dt, nd, whole, nest, first)
                })
                .unwrap();
            match h.join() {
                Ok((dt, nd, whole, nest, first)) => println!(
                    "depth {case} n={n} broken={broken} bytes={len} ok time={dt:?} diags={nd} whole={} nest={} first={:?}",
                    whole.as_deref().unwrap_or("ok"),
                    nest.as_deref().unwrap_or("ok"),
                    first
                ),
                Err(_) => println!("depth {case} n={n} broken={broken} PANIC"),
            }
        }
        "time" => {
            let shape = &args[2];
            let n: usize = args[3].parse().unwrap();
            let reps: usize = args[4].parse().unwrap();
            let text = hostile(shape, n);
            let vocab = symphonyscript_fixtures::vocabulary();
            let src = source(&text);
            let mut best = Duration::MAX;
            let mut nd = 0;
            let mut alloc = 0;
            let mut count = 0;
            let mut peak = 0;
            for _ in 0..reps {
                let a0 = ALLOCATED.load(Ordering::Relaxed);
                let c0 = COUNT.load(Ordering::Relaxed);
                let live0 = LIVE.load(Ordering::Relaxed);
                PEAK.store(live0, Ordering::Relaxed);
                let t0 = Instant::now();
                let p = syntax::parse(&src, &vocab);
                let dt = t0.elapsed();
                alloc = ALLOCATED.load(Ordering::Relaxed) - a0;
                count = COUNT.load(Ordering::Relaxed) - c0;
                peak = PEAK.load(Ordering::Relaxed) - live0;
                nd = p.diagnostics.len();
                drop(p);
                best = best.min(dt);
            }
            println!(
                "{shape} n={n} bytes={} min_ms={:.3} diags={nd} alloc_bytes={alloc} allocs={count} peak_bytes={peak}",
                text.len(),
                best.as_secs_f64() * 1e3
            );
        }
        "gen" => {
            print!("{}", hostile(&args[2], args[3].parse().unwrap()));
        }
        "depthgen" => {
            print!("{}", depth_source(&args[2], args[3].parse().unwrap(), args.get(4).is_some_and(|a| a == "broken")));
        }
        "show" => {
            let text = std::fs::read_to_string(&args[2]).unwrap();
            let vocab = symphonyscript_fixtures::vocabulary();
            show(&text, &vocab);
        }
        "showstr" => {
            let text = args[2].replace("\\n", "\n").replace("\\t", "\t");
            let vocab = symphonyscript_fixtures::vocabulary();
            show(&text, &vocab);
        }
        _ => {
            eprintln!("unknown command");
            std::process::exit(2);
        }
    }
}
