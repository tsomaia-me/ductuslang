use std::hint::black_box;
use std::time::{Duration, Instant};
use symphonyscript::Source;
use symphonyscript::syntax::{floor, parse};

fn read_pack(path: &str) -> Vec<Source> {
    let data = std::fs::read(path).unwrap();
    let mut at = 0;
    let mut out = Vec::new();
    while at + 4 <= data.len() {
        let len = u32::from_le_bytes(data[at..at + 4].try_into().unwrap()) as usize;
        at += 4;
        out.push(Source { path: format!("f{}.sym", out.len()), text: String::from_utf8(data[at..at + len].to_vec()).unwrap() });
        at += len;
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let voc = symphonyscript_fixtures::vocabulary();
    let filter = args[2].as_str();
    let sources: Vec<Source> = read_pack(&args[1]).into_iter().filter(|s| {
        let clean = parse(s, &voc).diagnostics.is_empty();
        match filter { "clean" => clean, "faulty" => !clean, _ => true }
    }).collect();
    let rounds: usize = args[3].parse().unwrap();
    let (mut bp, mut bi, mut ip, mut ii, mut it) = (vec![], vec![], vec![], vec![], vec![]);
    for round in 0..rounds + 2 {
        // batch, as floor.rs
        let mut kept = Vec::with_capacity(sources.len());
        let s = Instant::now();
        for src in &sources { kept.push(parse(black_box(src), &voc)); }
        let p = s.elapsed();
        let mut ir = floor::Ir::new();
        let s = Instant::now();
        for (src, parsed) in sources.iter().zip(&kept) { ir.add(&src.path, parsed, &voc); }
        black_box(ir.nodes());
        let i = s.elapsed();
        drop(ir); drop(kept);
        // interleaved, as compiler.rs
        let mut ir = floor::Ir::new();
        let (mut pp, mut pi) = (Duration::ZERO, Duration::ZERO);
        let total = Instant::now();
        for src in &sources {
            let s = Instant::now();
            let parsed = parse(black_box(src), &voc);
            let m = Instant::now();
            ir.add(&src.path, &parsed, &voc);
            pi += m.elapsed();
            pp += m - s;
            drop(parsed);
        }
        let t = total.elapsed();
        black_box(ir.nodes());
        drop(ir);
        if round >= 2 { bp.push(p); bi.push(i); ip.push(pp); ii.push(pi); it.push(t); }
    }
    let min = |v: &Vec<Duration>| v.iter().min().unwrap().as_secs_f64() * 1e3;
    println!("{}\t{}\tfiles={}\tbatch parse={:.3} ir={:.3} sum={:.3}\tinterleaved parse={:.3} ir={:.3} total(incl drops)={:.3}",
        args[1], filter, sources.len(), min(&bp), min(&bi), min(&bp) + min(&bi), min(&ip), min(&ii), min(&it));
}
