
// The peer's check of the trial. The printer recurses and is used on small trees only.
#[cfg(test)]
mod check {
    use super::*;

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
            "[" => TokKind::OpenSq,
            "]" => TokKind::CloseSq,
            "," => TokKind::Comma,
            "=" => TokKind::Assign,
            ".." => TokKind::DotDot,
            "if" => TokKind::If,
            "then" => TokKind::Then,
            "else" => TokKind::Else,
            w if w.chars().all(|c| c.is_ascii_digit()) => TokKind::Number,
            _ => TokKind::Name,
        }
    }

    fn show(tree: &Tree, words: &[&str], id: NodeId, out: &mut String) {
        let opt = |tree: &Tree, n: Option<NodeId>, out: &mut String| match n {
            Some(n) => show(tree, words, n, out),
            None => out.push('_'),
        };
        match tree.node(id) {
            Node::Leaf(t) => out.push_str(words[t.0]),
            Node::Prefix { op, operand } => {
                out.push_str(&format!("({} ", if words[op.0] == "-" { "neg" } else { words[op.0] }));
                show(tree, words, operand, out);
                out.push(')');
            }
            Node::Infix { op, left, right } => {
                out.push_str(&format!("({} ", words[op.0]));
                show(tree, words, left, out);
                out.push(' ');
                show(tree, words, right, out);
                out.push(')');
            }
            Node::Group { inside } => {
                out.push_str("(group ");
                show(tree, words, inside, out);
                out.push(')');
            }
            Node::If { condition, then, otherwise } => {
                out.push_str("(if ");
                show(tree, words, condition, out);
                out.push(' ');
                show(tree, words, then, out);
                out.push(' ');
                show(tree, words, otherwise, out);
                out.push(')');
            }
            Node::Call { callee, args } => {
                out.push_str("(call ");
                show(tree, words, callee, out);
                for &a in tree.parts(args) {
                    out.push(' ');
                    show(tree, words, a, out);
                }
                out.push(')');
            }
            Node::Named { name, value } => {
                out.push_str(&format!("{}=", words[name.0]));
                show(tree, words, value, out);
            }
            Node::Index { target, index } => {
                out.push_str("(index ");
                show(tree, words, target, out);
                out.push(' ');
                show(tree, words, index, out);
                out.push(')');
            }
            Node::Slice { target, lower, upper } => {
                out.push_str("(slice ");
                show(tree, words, target, out);
                out.push(' ');
                opt(tree, lower, out);
                out.push(' ');
                opt(tree, upper, out);
                out.push(')');
            }
            Node::Array { items } => {
                out.push_str("(array");
                for &a in tree.parts(items) {
                    out.push(' ');
                    show(tree, words, a, out);
                }
                out.push(')');
            }
            Node::Fault(f) => out.push_str(&format!("FAULT {:?} at {}", f.code, f.at.0)),
        }
    }

    fn run(src: &str) -> (String, usize) {
        let words: Vec<&str> = src.split_whitespace().collect();
        let toks = Toks::end(words.iter().map(|w| kind_of(w)).collect());
        let mut tree = Tree { nodes: Vec::new(), lists: Vec::new() };
        let mut scratch = Scratch { stack: Vec::new(), parts: Vec::new() };
        let e = expression(&toks, toks.first(), &mut tree, &mut scratch);
        assert!(scratch.stack.is_empty() && scratch.parts.is_empty(), "scratch not empty: {src}");
        let mut out = String::new();
        show(&tree, &words, e.root, &mut out);
        (out, e.next.0)
    }

    #[test]
    fn trees() {
        let cases: &[(&str, &str)] = &[
            ("a or b and c", "(or a (and b c))"),
            ("- 2 ** 2", "(neg (** 2 2))"),
            ("2 ** - 1", "(** 2 (neg 1))"),
            ("( a < b ) < c", "(< (group (< a b)) c)"),
            ("f ( )", "(call f)"),
            ("f ( a , b + 1 )", "(call f a (+ b 1))"),
            ("f ( x = 1 , y = a * 2 )", "(call f x=1 y=(* a 2))"),
            ("- f ( x ) ** 2", "(neg (** (call f x) 2))"),
            ("f ( a ) ( b )", "(call (call f a) b)"),
            ("a [ 1 ]", "(index a 1)"),
            ("a [ 1 .. ]", "(slice a 1 _)"),
            ("a [ .. b ]", "(slice a _ b)"),
            ("a [ .. ]", "(slice a _ _)"),
            ("a [ i + 1 .. j ] [ 0 ]", "(index (slice a (+ i 1) j) 0)"),
            ("[ ]", "(array)"),
            ("[ a , b , ]", "(array a b)"),
            ("[ a , [ b ] ] [ 0 ]", "(index (array a (array b)) 0)"),
            ("if a then b else c + 1", "(if a b (+ c 1))"),
            ("( if a then b else c ) + 1", "(+ (group (if a b c)) 1)"),
            ("if a then b else if c then d else e", "(if a b (if c d e))"),
            ("if if a then b else c then d else e", "(if (if a b c) d e)"),
            ("f ( if a then b else c , d )", "(call f (if a b c) d)"),
            ("[ if a then b else c ]", "(array (if a b c))"),
            ("f ( y = g ( [ 1 , 2 ] [ 0 ] ) , x = h ( ) )", "(call f y=(call g (index (array 1 2) 0)) x=(call h))"),
            ("a + b ) c", "(+ a b)"),
        ];
        for (src, want) in cases {
            assert_eq!(run(src).0, *want, "{src}");
        }
        assert_eq!(run("a + b ) c").1, 3);
        assert_eq!(run("if a then b else c then").1, 6);
    }

    #[test]
    fn faults() {
        let cases: &[(&str, &str)] = &[
            ("a < b < c", "FAULT ComparisonChain at 3"),
            ("a + not b", "FAULT OperandExpected at 2"),
            ("( )", "FAULT OperandExpected at 1"),
            ("( a b )", "FAULT CloserExpected at 2"),
            ("( a , b )", "FAULT CloserExpected at 2"),
            ("1 + if a then b else c", "FAULT UnparenthesizedIf at 2"),
            ("not if a then b else c", "FAULT UnparenthesizedIf at 1"),
            ("if a b", "FAULT ThenExpected at 2"),
            ("if a then b", "FAULT ElseExpected at 4"),
            ("if a then b ) c", "FAULT ElseExpected at 4"),
            ("if then", "FAULT OperandExpected at 1"),
            ("f ( a b )", "FAULT CloserExpected at 3"),
            ("f ( a , )", "FAULT OperandExpected at 4"),
            ("f ( , )", "FAULT OperandExpected at 2"),
            ("f ( x = 1 , 2 )", "FAULT MixedArguments at 7"),
            ("f ( 1 , x = 2 )", "FAULT MixedArguments at 7"),
            ("f ( x + 1 = 2 )", "FAULT ArgumentName at 5"),
            ("f ( ( x ) = 2 )", "FAULT ArgumentName at 5"),
            ("f ( 1 = 2 )", "FAULT ArgumentName at 3"),
            ("f ( x = = 2 )", "FAULT OperandExpected at 4"),
            ("f ( x = y = 2 )", "FAULT ArgumentName at 5"),
            ("f ( x = )", "FAULT OperandExpected at 4"),
            ("a [ ]", "FAULT OperandExpected at 2"),
            ("a [ 1 , 2 ]", "FAULT CloserExpected at 3"),
            ("a [ 1 .. 2 .. 3 ]", "FAULT CloserExpected at 5"),
            ("a [ .. .. ]", "FAULT OperandExpected at 3"),
            ("[ , ]", "FAULT OperandExpected at 1"),
            ("[ a b ]", "FAULT CloserExpected at 2"),
            ("[ a , , ]", "FAULT OperandExpected at 3"),
            ("[ a )", "FAULT CloserExpected at 2"),
        ];
        for (src, want) in cases {
            assert_eq!(run(src).0, *want, "{src}");
        }
    }

    fn deep(kinds: Vec<TokKind>) -> (usize, usize) {
        let toks = Toks::end(kinds);
        let mut tree = Tree { nodes: Vec::new(), lists: Vec::new() };
        let mut scratch = Scratch { stack: Vec::new(), parts: Vec::new() };
        let e = expression(&toks, toks.first(), &mut tree, &mut scratch);
        assert!(!matches!(tree.node(e.root), Node::Fault(_)), "fault at depth");
        assert!(scratch.stack.is_empty() && scratch.parts.is_empty());
        (tree.nodes.len(), e.next.0)
    }

    #[test]
    fn hundred_thousand_levels_on_one_mebibyte() {
        const N: usize = 100_000;
        let handle = std::thread::Builder::new()
            .stack_size(1 << 20)
            .spawn(|| {
                use TokKind as T;
                let rep = |unit: &[TokKind], n: usize| -> Vec<TokKind> {
                    unit.iter().copied().cycle().take(unit.len() * n).collect()
                };
                // ((( … a … )))
                let mut groups = rep(&[T::Open], N);
                groups.push(T::Name);
                groups.extend(rep(&[T::Close], N));
                assert_eq!(deep(groups).0, N + 1);
                // f(f(f( … a … )))
                let mut calls = rep(&[T::Name, T::Open], N);
                calls.push(T::Name);
                calls.extend(rep(&[T::Close], N));
                assert_eq!(deep(calls).0, 2 * N + 1);
                // [[[ … a … ]]]
                let mut arrays = rep(&[T::OpenSq], N);
                arrays.push(T::Name);
                arrays.extend(rep(&[T::CloseSq], N));
                assert_eq!(deep(arrays).0, N + 1);
                // a[a[a[ … a … ]]]
                let mut indexes = rep(&[T::Name, T::OpenSq], N);
                indexes.push(T::Name);
                indexes.extend(rep(&[T::CloseSq], N));
                assert_eq!(deep(indexes).0, 2 * N + 1);
                // if a then b else if a then b else … c
                let mut chain = rep(&[T::If, T::Name, T::Then, T::Name, T::Else], N);
                chain.push(T::Name);
                assert_eq!(deep(chain).0, 3 * N + 1);
                // if if if … a then b else c then b else c …
                let mut conditions = rep(&[T::If], N);
                conditions.push(T::Name);
                conditions.extend(rep(&[T::Then, T::Name, T::Else, T::Name], N));
                assert_eq!(deep(conditions).0, 3 * N + 1);
                // f(x = f(x = … 1 … )): a level makes the callee's leaf, the name's leaf, the
                // named argument and the call.
                let mut named = rep(&[T::Name, T::Open, T::Name, T::Assign], N);
                named.push(T::Number);
                named.extend(rep(&[T::Close], N));
                assert_eq!(deep(named).0, 4 * N + 1);
            })
            .unwrap();
        handle.join().unwrap();
    }
}
