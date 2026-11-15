
// The peer's own check of the worked example. Not part of the standard.
#[cfg(test)]
mod check {
    use super::*;

    // Counts this thread's allocations, so that a test can say how many a call made.
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    struct Counting;

    thread_local! {
        static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    }

    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
            unsafe { System.realloc(ptr, layout, new_size) }
        }
    }

    #[global_allocator]
    static COUNTING: Counting = Counting;

    fn allocations() -> u64 {
        ALLOCATIONS.with(|c| c.get())
    }

    // 900,000 tokens: a + ( a * - a ) ** a + ( a * - a ) ** … a + a + a + a + a
    #[test]
    fn allocations_of_an_expression() {
        use TokKind as T;
        let unit = [T::Name, T::Plus, T::Open, T::Name, T::Star, T::Minus, T::Name, T::Close, T::Pow];
        let mut kinds: Vec<TokKind> = unit.iter().copied().cycle().take(unit.len() * 99_999).collect();
        kinds.extend([T::Name, T::Plus, T::Name, T::Plus, T::Name, T::Plus, T::Name, T::Plus, T::Name]);
        assert_eq!(kinds.len(), 900_000);
        let toks = Toks::end(kinds);

        // A tree and a stack that start empty grow by doubling.
        let mut tree = Tree { nodes: Vec::new() };
        let mut scratch = Scratch { stack: Vec::new(), turns: 0 };
        let before = allocations();
        let e = expression(&toks, toks.first(), &mut tree, &mut scratch);
        let cold = allocations() - before;
        assert!(!matches!(tree.node(e.root), Node::Fault(_)));
        assert_eq!(e.next.0, 900_000);
        let nodes = tree.nodes.len();
        println!("cold: {cold} allocations for {nodes} nodes");
        assert!(cold < 60, "{cold}");

        // The same stack again, and a tree with room: nothing is allocated.
        let mut tree = Tree { nodes: Vec::with_capacity(nodes) };
        let before = allocations();
        let e = expression(&toks, toks.first(), &mut tree, &mut scratch);
        let warm = allocations() - before;
        assert_eq!(tree.nodes.len(), nodes);
        assert_eq!(e.next.0, 900_000);
        println!("warm: {warm} allocations");
        assert_eq!(warm, 0);
    }

    fn kind_of(w: &str) -> TokKind {
        match w {
            "or" => TokKind::Or,
            "and" => TokKind::And,
            "not" => TokKind::Not,
            "<" => TokKind::Lt,
            "<=" => TokKind::Le,
            ">" => TokKind::Gt,
            ">=" => TokKind::Ge,
            "??" => TokKind::Fallback,
            "+" => TokKind::Plus,
            "-" => TokKind::Minus,
            "without" => TokKind::Without,
            "*" => TokKind::Star,
            "/" => TokKind::Slash,
            "div" => TokKind::Div,
            "%" => TokKind::Percent,
            "**" => TokKind::Pow,
            "(" => TokKind::Open,
            ")" => TokKind::Close,
            w if w.chars().all(|c| c.is_ascii_digit()) => TokKind::Number,
            _ => TokKind::Name,
        }
    }

    fn word_of(k: TokKind) -> &'static str {
        match k {
            TokKind::Name => "a",
            TokKind::Number => "1",
            TokKind::Or => "or",
            TokKind::And => "and",
            TokKind::Not => "not",
            TokKind::Lt => "<",
            TokKind::Le => "<=",
            TokKind::Gt => ">",
            TokKind::Ge => ">=",
            TokKind::Fallback => "??",
            TokKind::Plus => "+",
            TokKind::Minus => "-",
            TokKind::Without => "without",
            TokKind::Star => "*",
            TokKind::Slash => "/",
            TokKind::Div => "div",
            TokKind::Percent => "%",
            TokKind::Pow => "**",
            TokKind::Open => "(",
            TokKind::Close => ")",
        }
    }

    fn show(tree: &Tree, words: &[&str], id: NodeId, out: &mut String) {
        match tree.node(id) {
            Node::Leaf(t) => out.push_str(words[t.0]),
            Node::Prefix { op, operand } => {
                out.push('(');
                out.push_str(if words[op.0] == "-" { "neg" } else { words[op.0] });
                out.push(' ');
                show(tree, words, operand, out);
                out.push(')');
            }
            Node::Infix { op, left, right } => {
                out.push('(');
                out.push_str(words[op.0]);
                out.push(' ');
                show(tree, words, left, out);
                out.push(' ');
                show(tree, words, right, out);
                out.push(')');
            }
            Node::Fault(f) => out.push_str(&format!("FAULT {:?} at {}", f.code, f.at.0)),
        }
    }

    fn run_kinds(kinds: Vec<TokKind>, words: &[&str], scratch: &mut Scratch, tree: &mut Tree) -> (String, usize) {
        let toks = Toks::end(kinds);
        tree.nodes.clear();
        let e = expression(&toks, toks.first(), tree, scratch);
        assert!(scratch.stack.is_empty());
        let mut out = String::new();
        show(tree, words, e.root, &mut out);
        (out, e.next.0)
    }

    fn run(src: &str) -> (String, usize) {
        let words: Vec<&str> = src.split_whitespace().collect();
        let kinds: Vec<TokKind> = words.iter().map(|w| kind_of(w)).collect();
        let mut tree = Tree { nodes: Vec::new() };
        let mut scratch = Scratch { stack: Vec::new(), turns: 0 };
        run_kinds(kinds, &words, &mut scratch, &mut tree)
    }

    #[test]
    fn trees() {
        let cases: &[(&str, &str, usize)] = &[
            ("a", "a", 1),
            ("a or b and c", "(or a (and b c))", 5),
            ("a and b or c", "(or (and a b) c)", 5),
            ("a - b - c", "(- (- a b) c)", 5),
            ("a - b + c without d", "(without (+ (- a b) c) d)", 7),
            ("a * b / c div d % e", "(% (div (/ (* a b) c) d) e)", 9),
            ("a + b * c - d / e", "(- (+ a (* b c)) (/ d e))", 9),
            ("a ?? b ?? c", "(?? a (?? b c))", 5),
            ("a ** b ** c", "(** a (** b c))", 5),
            ("- 2 ** 2", "(neg (** 2 2))", 4),
            ("2 ** - 1", "(** 2 (neg 1))", 4),
            ("a ** - b ** c", "(** a (neg (** b c)))", 6),
            ("a ** b * c", "(* (** a b) c)", 5),
            ("- a * b", "(* (neg a) b)", 4),
            ("- a + b", "(+ (neg a) b)", 4),
            ("- - a", "(neg (neg a))", 3),
            ("not not a", "(not (not a))", 3),
            ("not - a", "(not (neg a))", 3),
            ("not a < b", "(not (< a b))", 4),
            ("not a and b", "(and (not a) b)", 4),
            ("a and not b or c", "(or (and a (not b)) c)", 6),
            ("a or not b", "(or a (not b))", 4),
            ("a * - b * c", "(* (* a (neg b)) c)", 6),
            ("a ?? b > c", "(> (?? a b) c)", 5),
            ("port ?? 80 > 1024", "(> (?? port 80) 1024)", 5),
            ("x ?? base + 1", "(?? x (+ base 1))", 5),
            ("x ?? - 1", "(?? x (neg 1))", 4),
            ("- x ?? 1", "(?? (neg x) 1)", 4),
            ("a + b ?? c", "(?? (+ a b) c)", 5),
            ("a < b and c >= d", "(and (< a b) (>= c d))", 7),
            ("a < b + c", "(< a (+ b c))", 5),
            ("( a < b ) < c", "(< (< a b) c)", 7),
            ("( a )", "a", 3),
            ("( ( a ) )", "a", 5),
            ("- ( a + b ) * c", "(* (neg (+ a b)) c)", 8),
            ("( not a ) + b", "(+ (not a) b)", 6),
            ("a * ( b + c )", "(* a (+ b c))", 7),
            ("a ** ( not b )", "(** a (not b))", 6),
            // where the expression stops and the caller goes on
            ("a )", "a", 1),
            ("a + b ) c", "(+ a b)", 3),
            ("( a ) b", "a", 3),
            ("a b", "a", 1),
            ("a not b", "a", 1),
            ("a ( b )", "a", 1),
        ];
        for (src, tree, next) in cases {
            assert_eq!(run(src), (tree.to_string(), *next), "{src}");
        }
    }

    #[test]
    fn faults() {
        let cases: &[(&str, &str)] = &[
            ("a < b < c", "FAULT ComparisonChain at 3"),
            ("a < b >= c", "FAULT ComparisonChain at 3"),
            ("a <= b > c", "FAULT ComparisonChain at 3"),
            ("a < b + c >= d", "FAULT ComparisonChain at 5"),
            ("a < b ?? c < d", "FAULT ComparisonChain at 5"),
            ("not a < b < c", "FAULT ComparisonChain at 4"),
            ("a + not b", "FAULT OperandExpected at 2"),
            ("a < not b", "FAULT OperandExpected at 2"),
            ("a ?? not b", "FAULT OperandExpected at 2"),
            ("a * not b", "FAULT OperandExpected at 2"),
            ("a ** not b", "FAULT OperandExpected at 2"),
            ("- not a", "FAULT OperandExpected at 1"),
            ("a +", "FAULT OperandExpected at 2"),
            ("", "FAULT OperandExpected at 0"),
            ("a + * b", "FAULT OperandExpected at 2"),
            ("( )", "FAULT OperandExpected at 1"),
            ("( a + )", "FAULT OperandExpected at 3"),
            ("( a b )", "FAULT CloserExpected at 2"),
            ("( a ( b ) )", "FAULT CloserExpected at 2"),
            ("( a", "FAULT Internal(\"brackets arrive paired\") at 2"),
        ];
        for (src, want) in cases {
            assert_eq!(run(src).0, *want, "{src}");
        }
    }

    // A second reader, written by recursion straight from §5.1's productions for the cut.
    // It answers in the same words as `run`, so the two can be compared on every sequence.
    struct Rd<'a> {
        k: &'a [TokKind],
        w: &'a [&'a str],
        p: usize,
    }

    type R = Result<String, String>;

    impl Rd<'_> {
        fn at(&self) -> Option<TokKind> {
            self.k.get(self.p).copied()
        }
        fn word(&self, i: usize) -> &str {
            self.w[i]
        }
        fn or_expr(&mut self) -> R {
            let mut l = self.and_expr()?;
            while let Some(TokKind::Or) = self.at() {
                self.p += 1;
                let r = self.and_expr()?;
                l = format!("(or {l} {r})");
            }
            Ok(l)
        }
        fn and_expr(&mut self) -> R {
            let mut l = self.not_expr()?;
            while let Some(TokKind::And) = self.at() {
                self.p += 1;
                let r = self.not_expr()?;
                l = format!("(and {l} {r})");
            }
            Ok(l)
        }
        fn not_expr(&mut self) -> R {
            if let Some(TokKind::Not) = self.at() {
                self.p += 1;
                let o = self.not_expr()?;
                return Ok(format!("(not {o})"));
            }
            self.cmp_expr()
        }
        fn is_cmp(k: TokKind) -> bool {
            matches!(k, TokKind::Lt | TokKind::Le | TokKind::Gt | TokKind::Ge)
        }
        fn cmp_expr(&mut self) -> R {
            let l = self.fallback()?;
            if !self.at().is_some_and(Self::is_cmp) {
                return Ok(l);
            }
            let op = self.p;
            self.p += 1;
            let r = self.fallback()?;
            if self.at().is_some_and(Self::is_cmp) {
                return Err(format!("FAULT ComparisonChain at {}", self.p));
            }
            Ok(format!("({} {l} {r})", self.word(op)))
        }
        fn fallback(&mut self) -> R {
            let l = self.add_expr()?;
            if let Some(TokKind::Fallback) = self.at() {
                self.p += 1;
                let r = self.fallback()?;
                return Ok(format!("(?? {l} {r})"));
            }
            Ok(l)
        }
        fn add_expr(&mut self) -> R {
            let mut l = self.mul_expr()?;
            while matches!(self.at(), Some(TokKind::Plus | TokKind::Minus | TokKind::Without)) {
                let op = self.p;
                self.p += 1;
                let r = self.mul_expr()?;
                l = format!("({} {l} {r})", self.word(op));
            }
            Ok(l)
        }
        fn mul_expr(&mut self) -> R {
            let mut l = self.unary()?;
            while matches!(self.at(), Some(TokKind::Star | TokKind::Slash | TokKind::Div | TokKind::Percent)) {
                let op = self.p;
                self.p += 1;
                let r = self.unary()?;
                l = format!("({} {l} {r})", self.word(op));
            }
            Ok(l)
        }
        fn unary(&mut self) -> R {
            if let Some(TokKind::Minus) = self.at() {
                self.p += 1;
                let o = self.unary()?;
                return Ok(format!("(neg {o})"));
            }
            self.pow()
        }
        fn pow(&mut self) -> R {
            let b = self.primary()?;
            if let Some(TokKind::Pow) = self.at() {
                self.p += 1;
                let e = self.unary()?;
                return Ok(format!("(** {b} {e})"));
            }
            Ok(b)
        }
        fn primary(&mut self) -> R {
            match self.at() {
                Some(TokKind::Name | TokKind::Number) => {
                    self.p += 1;
                    Ok(self.word(self.p - 1).to_string())
                }
                Some(TokKind::Open) => {
                    self.p += 1;
                    let inner = self.or_expr()?;
                    match self.at() {
                        Some(TokKind::Close) => {
                            self.p += 1;
                            Ok(inner)
                        }
                        None => Err(format!(
                            "FAULT Internal(\"brackets arrive paired\") at {}",
                            self.p
                        )),
                        _ => Err(format!("FAULT CloserExpected at {}", self.p)),
                    }
                }
                _ => Err(format!("FAULT OperandExpected at {}", self.p)),
            }
        }
    }

    fn reference(kinds: &[TokKind], words: &[&str]) -> (String, usize) {
        let mut rd = Rd { k: kinds, w: words, p: 0 };
        match rd.or_expr() {
            Ok(t) => (t, rd.p),
            Err(f) => {
                let at = rd.p;
                (f, at)
            }
        }
    }

    fn every_sequence(alphabet: &[TokKind], max_len: usize) -> u64 {
        let mut tree = Tree { nodes: Vec::new() };
        let mut scratch = Scratch { stack: Vec::new(), turns: 0 };
        let mut count = 0u64;
        for len in 0..=max_len {
            let mut idx = vec![0usize; len];
            loop {
                let kinds: Vec<TokKind> = idx.iter().map(|&i| alphabet[i]).collect();
                let words: Vec<&str> = kinds.iter().map(|&k| word_of(k)).collect();
                let want = reference(&kinds, &words);
                let got = run_kinds(kinds, &words, &mut scratch, &mut tree);
                assert_eq!(got, want, "{}", words.join(" "));
                count += 1;
                let mut i = 0;
                while i < len {
                    idx[i] += 1;
                    if idx[i] < alphabet.len() {
                        break;
                    }
                    idx[i] = 0;
                    i += 1;
                }
                if i == len {
                    break;
                }
            }
        }
        count
    }

    #[test]
    fn agrees_with_the_productions() {
        use TokKind::*;
        let all = [
            Name, Number, Or, And, Not, Lt, Le, Gt, Ge, Fallback, Plus, Minus, Without, Star,
            Slash, Div, Percent, Pow, Open, Close,
        ];
        let some = [Name, Or, And, Not, Lt, Fallback, Minus, Star, Pow, Open, Close];
        let n = every_sequence(&all, 5) + every_sequence(&some, 7);
        println!("sequences compared: {n}");
    }

    // Depth and growth on a 1 MiB stack: nothing recurses, and the turns grow in line.
    fn deep(shape: &str, n: usize) -> (u64, usize) {
        let mut kinds = Vec::new();
        match shape {
            "parens" => {
                kinds.extend(std::iter::repeat(TokKind::Open).take(n));
                kinds.push(TokKind::Name);
                kinds.extend(std::iter::repeat(TokKind::Close).take(n));
            }
            "neg" => {
                kinds.extend(std::iter::repeat(TokKind::Minus).take(n));
                kinds.push(TokKind::Name);
            }
            "pow" => {
                for _ in 0..n {
                    kinds.push(TokKind::Name);
                    kinds.push(TokKind::Pow);
                }
                kinds.push(TokKind::Name);
            }
            "add" => {
                for _ in 0..n {
                    kinds.push(TokKind::Name);
                    kinds.push(TokKind::Plus);
                }
                kinds.push(TokKind::Name);
            }
            "powneg" => {
                for _ in 0..n {
                    kinds.push(TokKind::Name);
                    kinds.push(TokKind::Pow);
                    kinds.push(TokKind::Minus);
                }
                kinds.push(TokKind::Name);
            }
            _ => panic!(),
        }
        let len = kinds.len();
        let toks = Toks::end(kinds);
        let mut tree = Tree { nodes: Vec::new() };
        let mut scratch = Scratch { stack: Vec::new(), turns: 0 };
        let e = expression(&toks, toks.first(), &mut tree, &mut scratch);
        assert!(!matches!(tree.node(e.root), Node::Fault(_)), "{shape}");
        assert_eq!(e.next.0, len);
        (scratch.turns, len)
    }

    #[test]
    fn depth_and_growth() {
        std::thread::Builder::new()
            .stack_size(1 << 20)
            .spawn(|| {
                for shape in ["parens", "neg", "pow", "add", "powneg"] {
                    let mut last = 0u64;
                    for n in [100_000usize, 200_000, 400_000, 800_000] {
                        let (turns, len) = deep(shape, n);
                        assert!(turns <= len as u64, "{shape} {n}: {turns} turns, {len} tokens");
                        if last != 0 {
                            let ratio = turns as f64 / last as f64;
                            assert!(ratio < 2.1, "{shape} {n}: ratio {ratio}");
                        }
                        last = turns;
                    }
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
