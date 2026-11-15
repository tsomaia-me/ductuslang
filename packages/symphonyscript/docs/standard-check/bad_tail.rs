
// The functions the document does not print, in the same style.
impl Parser<'_> {
    fn binary(&mut self, op: TokKind, l: usize, r: usize) -> usize {
        self.nodes.push(Node::Binary(op, l, r));
        self.nodes.len() - 1
    }
    fn unary(&mut self, op: TokKind, o: usize) -> usize {
        self.nodes.push(Node::Unary(op, o));
        self.nodes.len() - 1
    }
    fn parse_and(&mut self) -> usize {
        let mut left = self.parse_not();
        while self.toks[self.pos] == TokKind::And {
            self.pos += 1;
            let right = self.parse_not();
            left = self.binary(TokKind::And, left, right);
        }
        left
    }
    fn parse_not(&mut self) -> usize {
        if self.toks[self.pos] == TokKind::Not {
            self.pos += 1;
            let operand = self.parse_not();
            return self.unary(TokKind::Not, operand);
        }
        self.parse_cmp()
    }
    fn parse_fallback(&mut self) -> usize {
        let left = self.parse_add();
        if self.toks[self.pos] == TokKind::Fallback {
            self.pos += 1;
            let right = self.parse_fallback();
            return self.binary(TokKind::Fallback, left, right);
        }
        left
    }
    fn parse_add(&mut self) -> usize {
        let mut left = self.parse_mul();
        while matches!(self.toks[self.pos], TokKind::Plus | TokKind::Minus | TokKind::Without) {
            let op = self.toks[self.pos];
            self.pos += 1;
            let right = self.parse_mul();
            left = self.binary(op, left, right);
        }
        left
    }
    fn parse_mul(&mut self) -> usize {
        let mut left = self.parse_unary();
        while matches!(self.toks[self.pos], TokKind::Star | TokKind::Slash | TokKind::Div | TokKind::Percent) {
            let op = self.toks[self.pos];
            self.pos += 1;
            let right = self.parse_unary();
            left = self.binary(op, left, right);
        }
        left
    }
    fn show(&self, id: usize, words: &[&str], out: &mut String) {
        match self.nodes[id] {
            Node::Leaf(t) => out.push_str(words.get(t).copied().unwrap_or("<end>")),
            Node::Unary(op, o) => {
                out.push_str(&format!("{:?}(", op));
                self.show(o, words, out);
                out.push(')');
            }
            Node::Binary(op, l, r) => {
                out.push_str(&format!("{:?}(", op));
                self.show(l, words, out);
                out.push_str(", ");
                self.show(r, words, out);
                out.push(')');
            }
        }
    }
}

fn kind_of(w: &str) -> TokKind {
    match w {
        "or" => TokKind::Or, "and" => TokKind::And, "not" => TokKind::Not,
        "<" => TokKind::Lt, "<=" => TokKind::Le, ">" => TokKind::Gt, ">=" => TokKind::Ge,
        "??" => TokKind::Fallback, "+" => TokKind::Plus, "-" => TokKind::Minus,
        "without" => TokKind::Without, "*" => TokKind::Star, "/" => TokKind::Slash,
        "div" => TokKind::Div, "%" => TokKind::Percent, "**" => TokKind::Pow,
        "(" => TokKind::Open, ")" => TokKind::Close,
        w if w.chars().all(|c| c.is_ascii_digit()) => TokKind::Number,
        _ => TokKind::Name,
    }
}

fn run(src: &str) {
    let words: Vec<&str> = src.split_whitespace().collect();
    let mut toks: Vec<TokKind> = words.iter().map(|w| kind_of(w)).collect();
    toks.push(TokKind::End);
    let result = std::panic::catch_unwind(|| {
        let mut p = Parser { toks: &toks, pos: 0, nodes: Vec::new(), errors: Vec::new(), failed: false };
        let root = p.parse_or();
        let mut out = String::new();
        p.show(root, &words, &mut out);
        (out, p.pos, p.errors.clone())
    });
    match result {
        Ok((tree, pos, errors)) => println!(
            "{src:<22} -> {tree}; stopped at {pos} ({}); messages: {errors:?}",
            words.get(pos).copied().unwrap_or("<end>")
        ),
        Err(_) => println!("{src:<22} -> PANIC"),
    }
}

fn main() {
    let arg = std::env::args().nth(1);
    if let Some(shape) = arg {
        // Depth: run on a 1 MiB stack; the process dies if the stack overflows.
        let n = 100_000;
        let mut toks = Vec::new();
        match shape.as_str() {
            "neg" => { toks.extend(std::iter::repeat(TokKind::Minus).take(n)); toks.push(TokKind::Name); }
            "paren" => { toks.extend(std::iter::repeat(TokKind::Open).take(n)); toks.push(TokKind::Name); toks.extend(std::iter::repeat(TokKind::Close).take(n)); }
            "pow" => { for _ in 0..n { toks.push(TokKind::Name); toks.push(TokKind::Pow); } toks.push(TokKind::Name); }
            _ => panic!("shape"),
        }
        toks.push(TokKind::End);
        std::thread::Builder::new().stack_size(1 << 20).spawn(move || {
            let mut p = Parser { toks: &toks, pos: 0, nodes: Vec::new(), errors: Vec::new(), failed: false };
            p.parse_or();
            println!("{shape}: survived");
        }).unwrap().join().unwrap();
        return;
    }
    std::panic::set_hook(Box::new(|_| {}));
    for src in ["a + b * c", "a < b", "a < b < c", "a < b <= c", "( a b )", "( a b", "2 ** - 1", "a +", "a ** b ** c"] {
        run(src);
    }
}
