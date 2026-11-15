use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use symphonyscript::Source;
use symphonyscript::syntax::{floor, lex, parse};

struct Counting;

static ON: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);

fn note(size: u64) {
    ALLOCS.fetch_add(1, Ordering::Relaxed);
    BYTES.fetch_add(size, Ordering::Relaxed);
    let live = LIVE.fetch_add(size, Ordering::Relaxed) + size;
    PEAK.fetch_max(live, Ordering::Relaxed);
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ON.load(Ordering::Relaxed) {
            note(layout.size() as u64);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ON.load(Ordering::Relaxed) {
            LIVE.fetch_sub(layout.size() as u64, Ordering::Relaxed);
        }
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ON.load(Ordering::Relaxed) {
            LIVE.fetch_sub(layout.size() as u64, Ordering::Relaxed);
            note(new_size as u64);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn counted<T>(work: impl FnOnce() -> T) -> (T, u64, u64, u64) {
    ALLOCS.store(0, Ordering::Relaxed);
    BYTES.store(0, Ordering::Relaxed);
    LIVE.store(0, Ordering::Relaxed);
    PEAK.store(0, Ordering::Relaxed);
    ON.store(true, Ordering::Relaxed);
    let out = work();
    ON.store(false, Ordering::Relaxed);
    (
        out,
        ALLOCS.load(Ordering::Relaxed),
        BYTES.load(Ordering::Relaxed),
        PEAK.load(Ordering::Relaxed),
    )
}

fn read_pack(path: &str) -> Vec<Source> {
    let data = std::fs::read(path).expect("pack");
    let mut at = 0;
    let mut sources = Vec::new();
    while at + 4 <= data.len() {
        let len = u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize;
        at += 4;
        let text = String::from_utf8(data[at..at + len].to_vec()).expect("utf8");
        at += len;
        sources.push(Source {
            path: format!("f{}.sym", sources.len()),
            text,
        });
    }
    sources
}

fn stats(mut times: Vec<Duration>) -> (f64, f64) {
    times.sort();
    let min = times[0].as_secs_f64() * 1e9;
    let median = times[times.len() / 2].as_secs_f64() * 1e9;
    (min, median)
}

fn timed(budget: Duration, mut work: impl FnMut() -> Duration) -> (f64, f64, usize) {
    for _ in 0..2 {
        work();
    }
    let started = Instant::now();
    let mut times = Vec::new();
    while times.len() < 7 || (started.elapsed() < budget && times.len() < 400) {
        times.push(work());
    }
    let runs = times.len();
    let (min, median) = stats(times);
    (min, median, runs)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let pack = &args[1];
    let filter = args.get(2).map(String::as_str).unwrap_or("all");
    let budget = Duration::from_millis(
        args.get(3)
            .and_then(|text| text.parse().ok())
            .unwrap_or(1500),
    );
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let all = read_pack(pack);
    let sources: Vec<Source> = all
        .into_iter()
        .filter(|source| {
            let clean = parse(source, &vocabulary).diagnostics.is_empty();
            match filter {
                "clean" => clean,
                "faulty" => !clean,
                _ => true,
            }
        })
        .collect();
    if sources.is_empty() {
        println!("{pack}\t{filter}\tno sources");
        return;
    }

    let bytes: usize = sources.iter().map(|source| source.text.len()).sum();
    let lines: usize = sources
        .iter()
        .map(|source| source.text.bytes().filter(|byte| *byte == b'\n').count())
        .sum();
    let mut tokens = 0;
    let mut significant = 0;
    let mut nodes = 0;
    let mut diagnostics = 0;
    let mut ir_nodes = 0;
    let mut ir_failed = 0;
    {
        let mut ir = floor::Ir::new();
        for source in &sources {
            let parsed = parse(source, &vocabulary);
            let (t, s, n) = floor::counts(&parsed);
            tokens += t;
            significant += s;
            nodes += n;
            diagnostics += parsed.diagnostics.len();
            if std::env::var("FLOOR_DIAG").is_ok() {
                for diagnostic in parsed.diagnostics.iter().take(4) {
                    let at = diagnostic.span.range.start as usize;
                    let line = source.text[..at].matches('\n').count() + 1;
                    eprintln!("{line}: {:?} {}", diagnostic.code, diagnostic.summary);
                }
            }
            if !ir.add(&source.path, &parsed, &vocabulary) {
                ir_failed += 1;
            }
        }
        ir_nodes = ir.nodes().max(ir_nodes);
    }

    let scan = timed(budget, || {
        let start = Instant::now();
        let mut count = 0usize;
        for source in &sources {
            count += black_box(source.text.as_bytes())
                .iter()
                .filter(|byte| **byte == b'\n')
                .count();
        }
        black_box(count);
        start.elapsed()
    });
    let chars = timed(budget, || {
        let start = Instant::now();
        let mut count = 0usize;
        for source in &sources {
            for c in black_box(source.text.as_str()).chars() {
                count += usize::from(c.is_alphanumeric());
            }
        }
        black_box(count);
        start.elapsed()
    });
    let copy = timed(budget, || {
        let start = Instant::now();
        let mut kept = Vec::with_capacity(sources.len());
        for source in &sources {
            kept.push(black_box(source.text.clone()));
        }
        let elapsed = start.elapsed();
        drop(kept);
        elapsed
    });
    let lex_columns = timed(budget, || {
        let start = Instant::now();
        let mut total = 0usize;
        for source in &sources {
            total += floor::lex_columns(black_box(source), &vocabulary).0;
        }
        black_box(total);
        start.elapsed()
    });
    let lex_public = timed(budget, || {
        let mut kept = Vec::with_capacity(sources.len());
        let start = Instant::now();
        for source in &sources {
            kept.push(lex(black_box(source), &vocabulary));
        }
        let elapsed = start.elapsed();
        drop(kept);
        elapsed
    });
    let mut ir_times = Vec::new();
    let mut drop_times = Vec::new();
    let parse_time = timed(budget, || {
        let mut kept = Vec::with_capacity(sources.len());
        let start = Instant::now();
        for source in &sources {
            kept.push(parse(black_box(source), &vocabulary));
        }
        let elapsed = start.elapsed();
        let mut ir = floor::Ir::new();
        let start = Instant::now();
        for (source, parsed) in sources.iter().zip(&kept) {
            ir.add(&source.path, parsed, &vocabulary);
        }
        black_box(ir.nodes());
        ir_times.push(start.elapsed());
        let start = Instant::now();
        drop(ir);
        drop(kept);
        drop_times.push(start.elapsed());
        elapsed
    });
    let ir_time = stats(ir_times.split_off(2));
    let drop_time = stats(drop_times.split_off(2));

    let (_, lex_allocs, lex_bytes, lex_peak) = counted(|| {
        let mut total = 0usize;
        for source in &sources {
            total += floor::lex_columns(source, &vocabulary).0;
        }
        total
    });
    let (kept, parse_allocs, parse_bytes, parse_peak) = counted(|| {
        let mut kept = Vec::with_capacity(sources.len());
        for source in &sources {
            kept.push(parse(source, &vocabulary));
        }
        kept
    });
    let (ir, ir_allocs, ir_bytes, ir_peak) = counted(|| {
        let mut ir = floor::Ir::new();
        for (source, parsed) in sources.iter().zip(&kept) {
            ir.add(&source.path, parsed, &vocabulary);
        }
        ir
    });
    drop(ir);
    drop(kept);

    println!(
        "pack\t{pack}\t{filter}\tfiles={}\tbytes={bytes}\tlines={lines}\ttokens={tokens}\tsignificant={significant}\tnodes={nodes}\tir_nodes={ir_nodes}\tdiagnostics={diagnostics}\tir_failed={ir_failed}",
        sources.len()
    );
    let row = |name: &str, (min, median, runs): (f64, f64, usize)| {
        println!(
            "time\t{name}\tmin_ms={:.3}\tmedian_ms={:.3}\truns={runs}\tns_byte={:.2}\tns_token={:.1}\tns_sig={:.1}\tns_node={:.1}\tns_line={:.0}\tmb_s={:.1}",
            min / 1e6,
            median / 1e6,
            min / bytes as f64,
            min / tokens as f64,
            min / significant as f64,
            min / nodes as f64,
            min / lines.max(1) as f64,
            bytes as f64 / (min / 1e9) / 1e6,
        );
    };
    row("scan_newlines", scan);
    row("chars_pass", chars);
    row("copy_text", copy);
    row("lex_columns", lex_columns);
    row("lex_public", lex_public);
    row("parse", parse_time);
    row("ir_build", (ir_time.0, ir_time.1, 0));
    row("drop_all", (drop_time.0, drop_time.1, 0));
    let mem = |name: &str, allocs: u64, total: u64, peak: u64| {
        println!(
            "alloc\t{name}\tallocs={allocs}\tbytes={total}\tpeak={peak}\tallocs_per_file={:.1}\tpeak_per_node={:.1}\tpeak_per_byte={:.2}",
            allocs as f64 / sources.len() as f64,
            peak as f64 / nodes as f64,
            peak as f64 / bytes as f64,
        );
    };
    mem("lex_columns", lex_allocs, lex_bytes, lex_peak);
    mem("parse", parse_allocs, parse_bytes, parse_peak);
    mem("ir_build", ir_allocs, ir_bytes, ir_peak);
}
