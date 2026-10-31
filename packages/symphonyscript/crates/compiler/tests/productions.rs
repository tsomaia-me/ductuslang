mod support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use symphonyscript_fixtures::{PENDING_PRODUCTIONS, pending_lines, spec_productions};

const CASES: &[(&str, &str, &str)] = &[
    (
        "File",
        "state a(1)\nClip c\n",
        "File[CellDecl[state a Init[( Literal[1] )] NL] Placement[Clip c NL]]",
    ),
    (
        "TopItem",
        "state a(1)\nlet b = 2\nstate s = 3\nfn f(x): x\npattern P():\n    Note\nClip c\n",
        "File[CellDecl[state a Init[( Literal[1] )] NL] LetDecl[let b = Literal[2] NL] CellDecl[state s = Literal[3] NL] FnDecl[fn f ParamList[( Param[x] )] : FnBody[NameRef[x] NL]] PatternDecl[pattern P PatternParamList[( )] : Block[NL > Placement[Note NL] <]] Placement[Clip c NL]]",
    ),
    (
        "NumberLit",
        "let a = [5, 0.5, 2bars, 0x1F]\n",
        "File[LetDecl[let a = ArrayLit[[ Literal[5] , Literal[0.5] , Literal[2bars] , Literal[0x1F] ]] NL]]",
    ),
    (
        "PrefixedInt",
        "let a = 0x1F + 0b101 + 0o17\n",
        "File[LetDecl[let a = BinaryExpr[BinaryExpr[Literal[0x1F] + Literal[0b101]] + Literal[0o17]] NL]]",
    ),
    (
        "FloatLit",
        "let a = 0.5 + 2e3 + 1.5e-3\n",
        "File[LetDecl[let a = BinaryExpr[BinaryExpr[Literal[0.5] + Literal[2e3]] + Literal[1.5e-3]] NL]]",
    ),
    (
        "Exponent",
        "let a = 2E+3 + 1e-2 + 3e4\n",
        "File[LetDecl[let a = BinaryExpr[BinaryExpr[Literal[2E+3] + Literal[1e-2]] + Literal[3e4]] NL]]",
    ),
    (
        "Digits",
        "let a = 1_000_000\n",
        "File[LetDecl[let a = Literal[1_000_000] NL]]",
    ),
    (
        "HexDigits",
        "let a = 0xDEAD_beef\n",
        "File[LetDecl[let a = Literal[0xDEAD_beef] NL]]",
    ),
    (
        "BinDigits",
        "let a = 0b1010_0101\n",
        "File[LetDecl[let a = Literal[0b1010_0101] NL]]",
    ),
    (
        "OctDigits",
        "let a = 0o7_55\n",
        "File[LetDecl[let a = Literal[0o7_55] NL]]",
    ),
    (
        "Digit",
        "let a = 1234567890\n",
        "File[LetDecl[let a = Literal[1234567890] NL]]",
    ),
    (
        "HexDigit",
        "let a = 0x0123456789abcdefABCDEF\n",
        "File[LetDecl[let a = Literal[0x0123456789abcdefABCDEF] NL]]",
    ),
    (
        "BinDigit",
        "let a = 0b01\n",
        "File[LetDecl[let a = Literal[0b01] NL]]",
    ),
    (
        "OctDigit",
        "let a = 0o01234567\n",
        "File[LetDecl[let a = Literal[0o01234567] NL]]",
    ),
    (
        "Suffix",
        "let a = 5i64 + 200ms\n",
        "File[LetDecl[let a = BinaryExpr[Literal[5i64] + Literal[200ms]] NL]]",
    ),
    (
        "TypeSuffix",
        "let a = [5u8, 5i32, 5i64, 0.5f32, 0.5f64]\n",
        "File[LetDecl[let a = ArrayLit[[ Literal[5u8] , Literal[5i32] , Literal[5i64] , Literal[0.5f32] , Literal[0.5f64] ]] NL]]",
    ),
    (
        "UnitSuffix",
        "let a = 2bars + 1beats + 0.5s\n",
        "File[LetDecl[let a = BinaryExpr[BinaryExpr[Literal[2bars] + Literal[1beats]] + Literal[0.5s]] NL]]",
    ),
    (
        "BoolLit",
        "let a = true or false\n",
        "File[LetDecl[let a = BinaryExpr[Literal[true] or Literal[false]] NL]]",
    ),
    (
        "StringLit",
        "let a = \"kit/{i}.wav\"\n",
        "File[LetDecl[let a = StringLit[\" kit/ Interpolation[{ NameRef[i] }] .wav \"] NL]]",
    ),
    (
        "Char",
        "let a = \"any } text // here\"\n",
        "File[LetDecl[let a = StringLit[\" any } text // here \"] NL]]",
    ),
    (
        "Escape",
        "let a = \"\\\" \\\\ \\n \\t \\{\"\n",
        "File[LetDecl[let a = StringLit[\" \\\" \\\\ \\n \\t \\{ \"] NL]]",
    ),
    (
        "Type",
        "state a(0): f32[2][3]\nstate w(Wave::Saw): Wave\nstate h([]): u8[2][..4]\n",
        "File[CellDecl[state a Init[( Literal[0] )] : TypeRef[f32 [ 2 ] [ 3 ]] NL] CellDecl[state w Init[( VariantRef[Wave :: Saw] )] : TypeRef[Wave] NL] CellDecl[state h Init[( ArrayLit[[ ]] )] : TypeRef[u8 [ 2 ] [ .. 4 ]] NL]]",
    ),
    (
        "ScalarType",
        "state z(0): u8\nstate a(0): i32\nstate b(0): i64\nstate c(0): f32\nstate d(0): f64\nstate e(true): bool\n",
        "File[CellDecl[state z Init[( Literal[0] )] : TypeRef[u8] NL] CellDecl[state a Init[( Literal[0] )] : TypeRef[i32] NL] CellDecl[state b Init[( Literal[0] )] : TypeRef[i64] NL] CellDecl[state c Init[( Literal[0] )] : TypeRef[f32] NL] CellDecl[state d Init[( Literal[0] )] : TypeRef[f64] NL] CellDecl[state e Init[( Literal[true] )] : TypeRef[bool] NL]]",
    ),
    (
        "Expr",
        "let a = if c then 1 else 2\nlet b = x or y\n",
        "File[LetDecl[let a = IfExpr[if NameRef[c] then Literal[1] else Literal[2]] NL] LetDecl[let b = BinaryExpr[NameRef[x] or NameRef[y]] NL]]",
    ),
    (
        "ForExpr",
        "let a = for x in xs into s from 0: s + x\nlet b = 1 + (for i, x in xs into s from 0: s + i * x)\n",
        "File[LetDecl[let a = ForExpr[ForExprHead[for x in NameRef[xs] into s from Literal[0] :] BinaryExpr[NameRef[s] + NameRef[x]]] NL] LetDecl[let b = BinaryExpr[Literal[1] + ParenExpr[( ForExpr[ForExprHead[for i , x in NameRef[xs] into s from Literal[0] :] BinaryExpr[NameRef[s] + BinaryExpr[NameRef[i] * NameRef[x]]]] )]] NL]]",
    ),
    (
        "ForExprBlock",
        "state h = for x in xs into k from []:\n    let keep = x > 60\n    if keep then k + [x] else k\nfn f(xs):\n    for x in xs into n from 0:\n        n + x\n",
        "File[CellDecl[state h = ForExprBlock[ForExprHead[for x in NameRef[xs] into k from ArrayLit[[ ]] :] NL > LocalLet[let keep = BinaryExpr[NameRef[x] > Literal[60]] NL] IfExpr[if NameRef[keep] then BinaryExpr[NameRef[k] + ArrayLit[[ NameRef[x] ]]] else NameRef[k]] NL <]] FnDecl[fn f ParamList[( Param[xs] )] : FnBody[NL > ForExprBlock[ForExprHead[for x in NameRef[xs] into n from Literal[0] :] NL > BinaryExpr[NameRef[n] + NameRef[x]] NL <] <]]]",
    ),
    (
        "ForExprHead",
        "let a = for i: i32, x: f32 in 0..4 into s: f32 from 0.0: s + x\n",
        "File[LetDecl[let a = ForExpr[ForExprHead[for i : TypeRef[i32] , x : TypeRef[f32] in RangeExpr[Literal[0] .. Literal[4]] into s : TypeRef[f32] from Literal[0.0] :] BinaryExpr[NameRef[s] + NameRef[x]]] NL]]",
    ),
    (
        "IfExpr",
        "let a = if c then 1 else if d then 2 else 3\n",
        "File[LetDecl[let a = IfExpr[if NameRef[c] then Literal[1] else IfExpr[if NameRef[d] then Literal[2] else Literal[3]]] NL]]",
    ),
    (
        "OrExpr",
        "let a = x or y or z\n",
        "File[LetDecl[let a = BinaryExpr[BinaryExpr[NameRef[x] or NameRef[y]] or NameRef[z]] NL]]",
    ),
    (
        "AndExpr",
        "let a = x and y and not z\n",
        "File[LetDecl[let a = BinaryExpr[BinaryExpr[NameRef[x] and NameRef[y]] and PrefixExpr[not NameRef[z]]] NL]]",
    ),
    (
        "NotExpr",
        "let a = not not x is y\n",
        "File[LetDecl[let a = PrefixExpr[not PrefixExpr[not BinaryExpr[NameRef[x] is NameRef[y]]]] NL]]",
    ),
    (
        "CmpExpr",
        "let a = x + 1 < y\n",
        "File[LetDecl[let a = BinaryExpr[BinaryExpr[NameRef[x] + Literal[1]] < NameRef[y]] NL]]",
    ),
    (
        "CmpOp",
        "let a = [x is y, x is not y, x < y, x <= y, x > y, x >= y]\n",
        "File[LetDecl[let a = ArrayLit[[ BinaryExpr[NameRef[x] is NameRef[y]] , BinaryExpr[NameRef[x] is not NameRef[y]] , BinaryExpr[NameRef[x] < NameRef[y]] , BinaryExpr[NameRef[x] <= NameRef[y]] , BinaryExpr[NameRef[x] > NameRef[y]] , BinaryExpr[NameRef[x] >= NameRef[y]] ]] NL]]",
    ),
    (
        "AddExpr",
        "let a = 1 + 2 - 3 * 4\n",
        "File[LetDecl[let a = BinaryExpr[BinaryExpr[Literal[1] + Literal[2]] - BinaryExpr[Literal[3] * Literal[4]]] NL]]",
    ),
    (
        "MulExpr",
        "let a = 1 * 2 / 3 div 4 % 5\n",
        "File[LetDecl[let a = BinaryExpr[BinaryExpr[BinaryExpr[BinaryExpr[Literal[1] * Literal[2]] / Literal[3]] div Literal[4]] % Literal[5]] NL]]",
    ),
    (
        "UnaryExpr",
        "let a = - -x\nlet b = -2 ** 2\n",
        "File[LetDecl[let a = PrefixExpr[- PrefixExpr[- NameRef[x]]] NL] LetDecl[let b = PrefixExpr[- BinaryExpr[Literal[2] ** Literal[2]]] NL]]",
    ),
    (
        "PowExpr",
        "let a = 2 ** 3 ** -1\n",
        "File[LetDecl[let a = BinaryExpr[Literal[2] ** BinaryExpr[Literal[3] ** PrefixExpr[- Literal[1]]]] NL]]",
    ),
    (
        "Postfix",
        "let a = f(1)[0].gain\n",
        "File[LetDecl[let a = FieldExpr[IndexExpr[CallExpr[NameRef[f] ArgList[( Literal[1] )]] [ Bound[Literal[0]] ]] . gain] NL]]",
    ),
    (
        "Index",
        "let b = xs[^1] + xs[1..^2] + xs[..2] + xs[3..] + xs[..]\n",
        "File[LetDecl[let b = BinaryExpr[BinaryExpr[BinaryExpr[BinaryExpr[IndexExpr[NameRef[xs] [ Bound[^ Literal[1]] ]] + IndexExpr[NameRef[xs] [ Bound[Literal[1]] .. Bound[^ Literal[2]] ]]] + IndexExpr[NameRef[xs] [ .. Bound[Literal[2]] ]]] + IndexExpr[NameRef[xs] [ Bound[Literal[3]] .. ]]] + IndexExpr[NameRef[xs] [ .. ]]] NL]]",
    ),
    (
        "Bound",
        "let c = xs[^n - 1]\n",
        "File[LetDecl[let c = IndexExpr[NameRef[xs] [ Bound[^ BinaryExpr[NameRef[n] - Literal[1]]] ]] NL]]",
    ),
    (
        "Primary",
        "let a = [1, true, \"s\", n, Wave::Saw, i32(x), [1], (2)]\nClip c:\n    Plays | gain=to.gain: c\n",
        "File[LetDecl[let a = ArrayLit[[ Literal[1] , Literal[true] , StringLit[\" s \"] , NameRef[n] , VariantRef[Wave :: Saw] , Conversion[i32 ( NameRef[x] )] , ArrayLit[[ Literal[1] ]] , ParenExpr[( Literal[2] )] ]] NL] Placement[Clip c Body[: Block[NL > WirePlacement[Plays Attrs[| AttrEntry[gain = FieldExpr[ToExpr[to] . gain]]] : Destination[NameRef[c] NL]] <]]]]",
    ),
    (
        "ArrayLit",
        "let a = [1, 2, 3,]\nlet b = [[1], [2]]\nlet c = []\n",
        "File[LetDecl[let a = ArrayLit[[ Literal[1] , Literal[2] , Literal[3] , ]] NL] LetDecl[let b = ArrayLit[[ ArrayLit[[ Literal[1] ]] , ArrayLit[[ Literal[2] ]] ]] NL] LetDecl[let c = ArrayLit[[ ]] NL]]",
    ),
    (
        "Args",
        "let a = f(1, 2) + g(x=1, y=2) + h()\n",
        "File[LetDecl[let a = BinaryExpr[BinaryExpr[CallExpr[NameRef[f] ArgList[( Literal[1] , Literal[2] )]] + CallExpr[NameRef[g] ArgList[( NamedArg[x = Literal[1]] , NamedArg[y = Literal[2]] )]]] + CallExpr[NameRef[h] ArgList[( )]]] NL]]",
    ),
    (
        "NamedArg",
        "let a = scale(degree=3, root=48)\n",
        "File[LetDecl[let a = CallExpr[NameRef[scale] ArgList[( NamedArg[degree = Literal[3]] , NamedArg[root = Literal[48]] )]] NL]]",
    ),
    (
        "MatchBlock",
        "let a = match n:\n    0: 1\n    else: 2\n",
        "File[LetDecl[let a = MatchBlock[match NameRef[n] : NL > MatchArm[Literal[0] : Literal[1] NL] ElseArm[else : Literal[2] NL] <]]]",
    ),
    (
        "MatchArm",
        "let a = match n:\n    -1: 0\n    0: 1\n    true: 2\n    Saw: 3\n    else: 4\n",
        "File[LetDecl[let a = MatchBlock[match NameRef[n] : NL > MatchArm[PrefixExpr[- Literal[1]] : Literal[0] NL] MatchArm[Literal[0] : Literal[1] NL] MatchArm[Literal[true] : Literal[2] NL] MatchArm[NameRef[Saw] : Literal[3] NL] ElseArm[else : Literal[4] NL] <]]]",
    ),
    (
        "ElseArm",
        "let a = match b:\n    true: 1\n    else: 0\n",
        "File[LetDecl[let a = MatchBlock[match NameRef[b] : NL > MatchArm[Literal[true] : Literal[1] NL] ElseArm[else : Literal[0] NL] <]]]",
    ),
    (
        "Conversion",
        "let a = u8(x) + i32(x) + i64(x) + f32(x) + f64(x)\n",
        "File[LetDecl[let a = BinaryExpr[BinaryExpr[BinaryExpr[BinaryExpr[Conversion[u8 ( NameRef[x] )] + Conversion[i32 ( NameRef[x] )]] + Conversion[i64 ( NameRef[x] )]] + Conversion[f32 ( NameRef[x] )]] + Conversion[f64 ( NameRef[x] )]] NL]]",
    ),
    (
        "LetDecl",
        "let a: i32 = 1\nlet b = match a:\n    0: 1\n    else: 2\n",
        "File[LetDecl[let a : TypeRef[i32] = Literal[1] NL] LetDecl[let b = MatchBlock[match NameRef[a] : NL > MatchArm[Literal[0] : Literal[1] NL] ElseArm[else : Literal[2] NL] <]]]",
    ),
    (
        "FnDecl",
        "fn f(): 1\nfn g(x):\n    x\nfn h(bounded): bounded\n",
        "File[FnDecl[fn f ParamList[( )] : FnBody[Literal[1] NL]] FnDecl[fn g ParamList[( Param[x] )] : FnBody[NL > NameRef[x] NL <]] FnDecl[fn h ParamList[( Param[bounded] )] : FnBody[NameRef[bounded] NL]]]",
    ),
    (
        "CellDecl",
        "state cutoff(0.5)\nstream[16] mic(0.0): f32\nstate b = a + 0.5\nstate c2 on b = a + b\nstate held(0.0) on gate = if gate then level else held.past(1)\nstate m = match k:\n    0: 1\n    else: 2\nstate h = for x in xs into k from []:\n    k + [x]\n",
        "File[CellDecl[state cutoff Init[( Literal[0.5] )] NL] CellDecl[stream [ 16 ] mic Init[( Literal[0.0] )] : TypeRef[f32] NL] CellDecl[state b = BinaryExpr[NameRef[a] + Literal[0.5]] NL] CellDecl[state c2 Clock[on NameRef[b]] = BinaryExpr[NameRef[a] + NameRef[b]] NL] CellDecl[state held Init[( Literal[0.0] )] Clock[on NameRef[gate]] = IfExpr[if NameRef[gate] then NameRef[level] else CallExpr[FieldExpr[NameRef[held] . past] ArgList[( Literal[1] )]]] NL] CellDecl[state m = MatchBlock[match NameRef[k] : NL > MatchArm[Literal[0] : Literal[1] NL] ElseArm[else : Literal[2] NL] <]] CellDecl[state h = ForExprBlock[ForExprHead[for x in NameRef[xs] into k from ArrayLit[[ ]] :] NL > BinaryExpr[NameRef[k] + ArrayLit[[ NameRef[x] ]]] NL <]]]",
    ),
    (
        "CellWord",
        "state a(1)\nstream b(2)\nchannel[16] c(3)\n",
        "File[CellDecl[state a Init[( Literal[1] )] NL] CellDecl[stream b Init[( Literal[2] )] NL] CellDecl[channel [ 16 ] c Init[( Literal[3] )] NL]]",
    ),
    (
        "Init",
        "state steps([true, false])\nstate fine(0): f32\nstate recent([]) = (recent.past(1) + [note])[^8..]\n",
        "File[CellDecl[state steps Init[( ArrayLit[[ Literal[true] , Literal[false] ]] )] NL] CellDecl[state fine Init[( Literal[0] )] : TypeRef[f32] NL] CellDecl[state recent Init[( ArrayLit[[ ]] )] = IndexExpr[ParenExpr[( BinaryExpr[CallExpr[FieldExpr[NameRef[recent] . past] ArgList[( Literal[1] )]] + ArrayLit[[ NameRef[note] ]]] )] [ Bound[^ Literal[8]] .. ]] NL]]",
    ),
    (
        "Clock",
        "state c3 on a = a + b\nstate c4 on (a, b) = a + b\nstate c5 on (a + 1) = b\nstate hits(0) on (hit, bar) = if bar.stepped then 0 else hits.past(1) + 1\n",
        "File[CellDecl[state c3 Clock[on NameRef[a]] = BinaryExpr[NameRef[a] + NameRef[b]] NL] CellDecl[state c4 Clock[on ( NameRef[a] , NameRef[b] )] = BinaryExpr[NameRef[a] + NameRef[b]] NL] CellDecl[state c5 Clock[on ( BinaryExpr[NameRef[a] + Literal[1]] )] = NameRef[b] NL] CellDecl[state hits Init[( Literal[0] )] Clock[on ( NameRef[hit] , NameRef[bar] )] = IfExpr[if FieldExpr[NameRef[bar] . stepped] then Literal[0] else BinaryExpr[CallExpr[FieldExpr[NameRef[hits] . past] ArgList[( Literal[1] )]] + Literal[1]]] NL]]",
    ),
    (
        "FnBody",
        "fn f(x):\n    let y = x\n    match y:\n        0: 1\n        else: 2\n",
        "File[FnDecl[fn f ParamList[( Param[x] )] : FnBody[NL > LocalLet[let y = NameRef[x] NL] MatchBlock[match NameRef[y] : NL > MatchArm[Literal[0] : Literal[1] NL] ElseArm[else : Literal[2] NL] <] <]]]",
    ),
    (
        "LocalLet",
        "fn f(x):\n    let y = x\n    let z = match y:\n        0: 1\n        else: 2\n    z\n",
        "File[FnDecl[fn f ParamList[( Param[x] )] : FnBody[NL > LocalLet[let y = NameRef[x] NL] LocalLet[let z = MatchBlock[match NameRef[y] : NL > MatchArm[Literal[0] : Literal[1] NL] ElseArm[else : Literal[2] NL] <]] NameRef[z] NL <]]]",
    ),
    (
        "Params",
        "fn f(a, b = 1, c = a): a\n",
        "File[FnDecl[fn f ParamList[( Param[a] , Param[b = Literal[1]] , Param[c = NameRef[a]] )] : FnBody[NameRef[a] NL]]]",
    ),
    (
        "Param",
        "fn f(a, b = a + 1): a\n",
        "File[FnDecl[fn f ParamList[( Param[a] , Param[b = BinaryExpr[NameRef[a] + Literal[1]]] )] : FnBody[NameRef[a] NL]]]",
    ),
    (
        "PatternDecl",
        "pattern P(default root, n = 2):\n    let k = n\n    Note/root\n",
        "File[PatternDecl[pattern P PatternParamList[( PatternParam[default root] , PatternParam[n = Literal[2]] )] : Block[NL > LocalLet[let k = NameRef[n] NL] Placement[Note DefaultArg[/ NameRef[root]] NL] <]]]",
    ),
    (
        "PatternParams",
        "pattern P(a, default b, c = 1):\n    Note\n",
        "File[PatternDecl[pattern P PatternParamList[( PatternParam[a] , PatternParam[default b] , PatternParam[c = Literal[1]] )] : Block[NL > Placement[Note NL] <]]]",
    ),
    (
        "PatternParam",
        "pattern P(default root = 60):\n    Note\n",
        "File[PatternDecl[pattern P PatternParamList[( PatternParam[default root = Literal[60]] )] : Block[NL > Placement[Note NL] <]]]",
    ),
    (
        "TopPlacement",
        "Clip as verse\nClip'+ chorus /1 when x | gain=1\nSession main /120:\n    Track\n",
        "File[Placement[Clip as verse NL] Placement[Clip Flags[' +] chorus DefaultArg[/ Literal[1]] Gate[when NameRef[x]] Attrs[| AttrEntry[gain = Literal[1]]] NL] Placement[Session main DefaultArg[/ Literal[120]] Body[: Block[NL > Placement[Track NL] <]]]]",
    ),
    (
        "Placement",
        "Clip c:\n    Note as hook /60 when x | velocity=1\n",
        "File[Placement[Clip c Body[: Block[NL > Placement[Note as hook DefaultArg[/ Literal[60]] Gate[when NameRef[x]] Attrs[| AttrEntry[velocity = Literal[1]]] NL] <]]]]",
    ),
    (
        "Kind",
        "Clip c:\n    Note\n",
        "File[Placement[Clip c Body[: Block[NL > Placement[Note NL] <]]]]",
    ),
    (
        "Body",
        "Clip c:\n    Note\nClip d: Note Note\n",
        "File[Placement[Clip c Body[: Block[NL > Placement[Note NL] <]]] Placement[Clip d Body[: Block[Members[Placement[Note] Placement[Note]] NL]]]]",
    ),
    (
        "BodyItem",
        "Clip c:\n    Note\n    Plays: c\n    for i in 0..2:\n        Note\n    when x:\n        Note\n",
        "File[Placement[Clip c Body[: Block[NL > Placement[Note NL] WirePlacement[Plays : Destination[NameRef[c] NL]] ForBlock[for i in RangeExpr[Literal[0] .. Literal[2]] : Block[NL > Placement[Note NL] <]] WhenBlock[when NameRef[x] : Block[NL > Placement[Note NL] <]] <]]]]",
    ),
    (
        "Line",
        "Clip c:\n    Note | velocity=1\n    Note:\n        Plays: c\n    (Note | velocity=1)\n    [Note Note]\n    Note (Note when x) [Note]\n",
        "File[Placement[Clip c Body[: Block[NL > Placement[Note Attrs[| AttrEntry[velocity = Literal[1]]] NL] Placement[Note Body[: Block[NL > WirePlacement[Plays : Destination[NameRef[c] NL]] <]]] Line[ParenPlacement[( Placement[Note Attrs[| AttrEntry[velocity = Literal[1]]]] )] NL] Line[Bundle[[ Placement[Note] Placement[Note] ]] NL] Line[Placement[Note] ParenPlacement[( Placement[Note Gate[when NameRef[x]]] )] Bundle[[ Placement[Note] ]] NL] <]]]]",
    ),
    (
        "Flags",
        "Clip c:\n    Note'!?*+~@$\n    Note**/60\n",
        "File[Placement[Clip c Body[: Block[NL > Placement[Note Flags[' ! ? * + ~ @ $] NL] Placement[Note Flags[**] DefaultArg[/ Literal[60]] NL] <]]]]",
    ),
    (
        "FlagChar",
        "Clip c:\n    Note' Note! Note? Note* Note+ Note~ Note@ Note$\n",
        "File[Placement[Clip c Body[: Block[NL > Line[Placement[Note Flags[']] Placement[Note Flags[!]] Placement[Note Flags[?]] Placement[Note Flags[*]] Placement[Note Flags[+]] Placement[Note Flags[~]] Placement[Note Flags[@]] Placement[Note Flags[$]] NL] <]]]]",
    ),
    (
        "DefaultArg",
        "Clip c:\n    Note/60\n    Note / (1 + 2)\n",
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ Literal[60]] NL] Placement[Note DefaultArg[/ ParenExpr[( BinaryExpr[Literal[1] + Literal[2]] )]] NL] <]]]]",
    ),
    (
        "Atom",
        "Clip c:\n    Note/-1\n    Note/true\n    Note/\"s\"\n    Note/a.b.c\n    Note/Wave::Saw\n    Plays/to.gain: c\n    Plays/to.hook.velocity: c\n",
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ PrefixExpr[- Literal[1]]] NL] Placement[Note DefaultArg[/ Literal[true]] NL] Placement[Note DefaultArg[/ StringLit[\" s \"]] NL] Placement[Note DefaultArg[/ FieldExpr[FieldExpr[NameRef[a] . b] . c]] NL] Placement[Note DefaultArg[/ VariantRef[Wave :: Saw]] NL] WirePlacement[Plays DefaultArg[/ FieldExpr[ToExpr[to] . gain]] : Destination[NameRef[c] NL]] WirePlacement[Plays DefaultArg[/ FieldExpr[FieldExpr[ToExpr[to] . hook] . velocity]] : Destination[NameRef[c] NL]] <]]]]",
    ),
    (
        "Attrs",
        "Clip c:\n    Note | velocity=1 accent !accent\n",
        "File[Placement[Clip c Body[: Block[NL > Placement[Note Attrs[| AttrEntry[velocity = Literal[1]] AttrEntry[accent] AttrEntry[! accent]] NL] <]]]]",
    ),
    (
        "AttrEntry",
        "Clip c:\n    Note | pitch=60 accent !accent\n",
        "File[Placement[Clip c Body[: Block[NL > Placement[Note Attrs[| AttrEntry[pitch = Literal[60]] AttrEntry[accent] AttrEntry[! accent]] NL] <]]]]",
    ),
    (
        "SelfDelimited",
        "Clip c:\n    Note [Note Note] (Note | velocity=1)\n",
        "File[Placement[Clip c Body[: Block[NL > Line[Placement[Note] Bundle[[ Placement[Note] Placement[Note] ]] ParenPlacement[( Placement[Note Attrs[| AttrEntry[velocity = Literal[1]]]] )] NL] <]]]]",
    ),
    (
        "Member",
        "Clip c:\n    Note as a /1 Note (Note when x)\n",
        "File[Placement[Clip c Body[: Block[NL > Line[Placement[Note as a DefaultArg[/ Literal[1]]] Placement[Note] ParenPlacement[( Placement[Note Gate[when NameRef[x]]] )] NL] <]]]]",
    ),
    (
        "InlineChildren",
        "Clip c: Note [Note] (Note | velocity=1)\n",
        "File[Placement[Clip c Body[: Block[Members[Placement[Note] Bundle[[ Placement[Note] ]] ParenPlacement[( Placement[Note Attrs[| AttrEntry[velocity = Literal[1]]]] )]] NL]]]]",
    ),
    (
        "Bundle",
        "Clip c:\n    [Note (Note when x) Note as top]\n    [for i in 0..2: Note]\n    [Note for i in [1, 2]: Note/i]\n",
        "File[Placement[Clip c Body[: Block[NL > Line[Bundle[[ Placement[Note] ParenPlacement[( Placement[Note Gate[when NameRef[x]]] )] Placement[Note as top] ]] NL] Line[Bundle[[ BundleFor[for i in RangeExpr[Literal[0] .. Literal[2]] : Placement[Note]] ]] NL] Line[Bundle[[ Placement[Note] BundleFor[for i in ArrayLit[[ Literal[1] , Literal[2] ]] : Placement[Note DefaultArg[/ NameRef[i]]]] ]] NL] <]]]]",
    ),
    (
        "BundleFor",
        "Clip c:\n    [for i in 0..2: Note/i Note]\n",
        "File[Placement[Clip c Body[: Block[NL > Line[Bundle[[ BundleFor[for i in RangeExpr[Literal[0] .. Literal[2]] : Placement[Note DefaultArg[/ NameRef[i]]] Placement[Note]] ]] NL] <]]]]",
    ),
    (
        "ForBlock",
        "Clip c:\n    for i in 0..4:\n        Note/i\n    for s in [a, b]: Note\n",
        "File[Placement[Clip c Body[: Block[NL > ForBlock[for i in RangeExpr[Literal[0] .. Literal[4]] : Block[NL > Placement[Note DefaultArg[/ NameRef[i]] NL] <]] ForBlock[for s in ArrayLit[[ NameRef[a] , NameRef[b] ]] : Block[Members[Placement[Note]] NL]] <]]]]",
    ),
    (
        "LoopHead",
        "Clip c:\n    for i: i32, p in xs:\n        Note/p\n    [for i, p: i32 in xs: Note/p]\nlet a = for k, v: f32 in 0..4 into s from 0: s + v\n",
        "File[Placement[Clip c Body[: Block[NL > ForBlock[for i : TypeRef[i32] , p in NameRef[xs] : Block[NL > Placement[Note DefaultArg[/ NameRef[p]] NL] <]] Line[Bundle[[ BundleFor[for i , p : TypeRef[i32] in NameRef[xs] : Placement[Note DefaultArg[/ NameRef[p]]]] ]] NL] <]]] LetDecl[let a = ForExpr[ForExprHead[for k , v : TypeRef[f32] in RangeExpr[Literal[0] .. Literal[4]] into s from Literal[0] :] BinaryExpr[NameRef[s] + NameRef[v]]] NL]]",
    ),
    (
        "Iterable",
        "Clip c:\n    for i in a..b + 1:\n        Note\n    for s in list: Note\n",
        "File[Placement[Clip c Body[: Block[NL > ForBlock[for i in RangeExpr[NameRef[a] .. BinaryExpr[NameRef[b] + Literal[1]]] : Block[NL > Placement[Note NL] <]] ForBlock[for s in NameRef[list] : Block[Members[Placement[Note]] NL]] <]]]]",
    ),
    (
        "WirePlacement",
        "Clip c:\n    Plays' as w /0.5 when x | transpose=1: c\n",
        "File[Placement[Clip c Body[: Block[NL > WirePlacement[Plays Flags['] as w DefaultArg[/ Literal[0.5]] Gate[when NameRef[x]] Attrs[| AttrEntry[transpose = Literal[1]]] : Destination[NameRef[c] NL]] <]]]]",
    ),
    (
        "Destination",
        "Clip c:\n    Plays: if x then a else b\n    Plays: match n:\n        0: a\n        else: b\n",
        "File[Placement[Clip c Body[: Block[NL > WirePlacement[Plays : Destination[IfExpr[if NameRef[x] then NameRef[a] else NameRef[b]] NL]] WirePlacement[Plays : Destination[MatchBlock[match NameRef[n] : NL > MatchArm[Literal[0] : NameRef[a] NL] ElseArm[else : NameRef[b] NL] <]]] <]]]]",
    ),
    (
        "Gate",
        "Clip c:\n    Note when x and y\n",
        "File[Placement[Clip c Body[: Block[NL > Placement[Note Gate[when BinaryExpr[NameRef[x] and NameRef[y]]] NL] <]]]]",
    ),
    (
        "WhenBlock",
        "Clip c:\n    when x: Note\n    else: Note\n    when:\n        a: Note\n        else: Note\n",
        "File[Placement[Clip c Body[: Block[NL > WhenBlock[when NameRef[x] : Block[Members[Placement[Note]] NL] WhenElse[else : Block[Members[Placement[Note]] NL]]] WhenBlock[when : NL > GuardArm[NameRef[a] : Block[Members[Placement[Note]] NL]] WhenElse[else : Block[Members[Placement[Note]] NL]] <] <]]]]",
    ),
    (
        "GuardArm",
        "Clip c:\n    when:\n        a is 0: Note\n        b:\n            Note\n",
        "File[Placement[Clip c Body[: Block[NL > WhenBlock[when : NL > GuardArm[BinaryExpr[NameRef[a] is Literal[0]] : Block[Members[Placement[Note]] NL]] GuardArm[NameRef[b] : Block[NL > Placement[Note NL] <]] <] <]]]]",
    ),
    (
        "Block",
        "Clip c:\n    when x: Note\n    else:\n        Note\n",
        "File[Placement[Clip c Body[: Block[NL > WhenBlock[when NameRef[x] : Block[Members[Placement[Note]] NL] WhenElse[else : Block[NL > Placement[Note NL] <]]] <]]]]",
    ),
    (
        "JsonLit",
        "let a = json {\"a\": 1, \"b\": [true, null]}\n",
        "File[LetDecl[let a = JsonLit[json JsonObject[{ JsonEntry[\"a\" : JsonAtom[1]] , JsonEntry[\"b\" : JsonArray[[ JsonAtom[true] , JsonAtom[null] ]]] }]] NL]]",
    ),
    (
        "JsonValue",
        "let a = json [{}, [], \"s\", -1, 2.5, true, false, null, n, n.x, Wave::Saw, to.gain, (n + 1)]\n",
        "File[LetDecl[let a = JsonLit[json JsonArray[[ JsonObject[{ }] , JsonArray[[ ]] , JsonAtom[\"s\"] , JsonAtom[- 1] , JsonAtom[2.5] , JsonAtom[true] , JsonAtom[false] , JsonAtom[null] , NameRef[n] , FieldExpr[NameRef[n] . x] , VariantRef[Wave :: Saw] , FieldExpr[ToExpr[to] . gain] , ParenExpr[( BinaryExpr[NameRef[n] + Literal[1]] )] ]]] NL]]",
    ),
    (
        "JsonObject",
        "let a = json {\"k\": {\"j\": [1, {\"i\": []}]}}\n",
        "File[LetDecl[let a = JsonLit[json JsonObject[{ JsonEntry[\"k\" : JsonObject[{ JsonEntry[\"j\" : JsonArray[[ JsonAtom[1] , JsonObject[{ JsonEntry[\"i\" : JsonArray[[ ]]] }] ]]] }]] }]] NL]]",
    ),
    (
        "JsonArray",
        "let a = json [[1, 2], [], [[3]]]\n",
        "File[LetDecl[let a = JsonLit[json JsonArray[[ JsonArray[[ JsonAtom[1] , JsonAtom[2] ]] , JsonArray[[ ]] , JsonArray[[ JsonArray[[ JsonAtom[3] ]] ]] ]]] NL]]",
    ),
    (
        "JsonString",
        "let a = json [\"\", \"plain\", \"a{b}\"]\n",
        "File[LetDecl[let a = JsonLit[json JsonArray[[ JsonAtom[\"\"] , JsonAtom[\"plain\"] , JsonAtom[\"a{b}\"] ]]] NL]]",
    ),
    (
        "JsonChar",
        "let a = json \"any text: { } [ ] // here\"\n",
        "File[LetDecl[let a = JsonLit[json JsonAtom[\"any text: { } [ ] // here\"]] NL]]",
    ),
    (
        "JsonEscape",
        "let a = json \"\\\" \\\\ \\/ \\b \\f \\n \\r \\t \\u00e9\"\n",
        "File[LetDecl[let a = JsonLit[json JsonAtom[\"\\\" \\\\ \\/ \\b \\f \\n \\r \\t \\u00e9\"]] NL]]",
    ),
    (
        "JsonNumber",
        "let a = json [0, 7, 120, -3, 0.5, 1e3, 2E-2, 6.02e+23]\n",
        "File[LetDecl[let a = JsonLit[json JsonArray[[ JsonAtom[0] , JsonAtom[7] , JsonAtom[120] , JsonAtom[- 3] , JsonAtom[0.5] , JsonAtom[1e3] , JsonAtom[2E-2] , JsonAtom[6.02e+23] ]]] NL]]",
    ),
    (
        "FnType",
        "fn apply(f: (i32, x: f64) -> bool, g: () -> i32): 1\n",
        "File[FnDecl[fn apply ParamList[( Param[f : FnType[( FnParam[TypeRef[i32]] , FnParam[x : TypeRef[f64]] ) -> TypeRef[bool]]] , Param[g : FnType[( ) -> TypeRef[i32]]] )] : FnBody[Literal[1] NL]]]",
    ),
    (
        "FnParam",
        "fn apply(f: (x: f64) -> f64, g: (i32) -> i32): 1\n",
        "File[FnDecl[fn apply ParamList[( Param[f : FnType[( FnParam[x : TypeRef[f64]] ) -> TypeRef[f64]]] , Param[g : FnType[( FnParam[TypeRef[i32]] ) -> TypeRef[i32]]] )] : FnBody[Literal[1] NL]]]",
    ),
    (
        "Base",
        "let a: (i32?)[..4] = []\nlet b: i32[..4]? = []\nlet c: Wave = Wave::Saw\nlet d: none = 1\n",
        "File[LetDecl[let a : TypeRef[ParenType[( TypeRef[i32 ?] )] [ .. 4 ]] = ArrayLit[[ ]] NL] LetDecl[let b : TypeRef[i32 [ .. 4 ] ?] = ArrayLit[[ ]] NL] LetDecl[let c : TypeRef[Wave] = VariantRef[Wave :: Saw] NL] LetDecl[let d : TypeRef[none] = Literal[1] NL]]",
    ),
    (
        "Dims",
        "let a: i32[3][..10] = []\nlet b: i32[..4][..10] = []\n",
        "File[LetDecl[let a : TypeRef[i32 [ 3 ] [ .. 10 ]] = ArrayLit[[ ]] NL] LetDecl[let b : TypeRef[i32 [ .. 4 ] [ .. 10 ]] = ArrayLit[[ ]] NL]]",
    ),
    (
        "Size",
        "let a: T[..N + 1] = []\nlet b: T[N * 2] = []\n",
        "File[LetDecl[let a : TypeRef[T [ .. Size[BinaryExpr[NameRef[N] + Literal[1]]] ]] = ArrayLit[[ ]] NL] LetDecl[let b : TypeRef[T [ Size[BinaryExpr[NameRef[N] * Literal[2]]] ]] = ArrayLit[[ ]] NL]]",
    ),
    (
        "StrType",
        "let c: str[..32][4] = []\nlet d: str = 1\n",
        "File[LetDecl[let c : TypeRef[StrType[str [ .. 32 ]] [ 4 ]] = ArrayLit[[ ]] NL] LetDecl[let d : TypeRef[StrType[str]] = Literal[1] NL]]",
    ),
    (
        "MapType",
        "let m: {str[..32]: i32}[..64] = 1\n",
        "File[LetDecl[let m : TypeRef[MapType[{ TypeRef[StrType[str [ .. 32 ]]] : TypeRef[i32] } [ .. 64 ]]] = Literal[1] NL]]",
    ),
    (
        "JsonType",
        "let j: json[..4096] = 1\nlet k: json = 1\n",
        "File[LetDecl[let j : TypeRef[JsonType[json [ .. 4096 ]]] = Literal[1] NL] LetDecl[let k : TypeRef[JsonType[json]] = Literal[1] NL]]",
    ),
    (
        "TypeDecl",
        "type Chord:\n    root: i32\n    spread: i32\ntype Shape = Circle | Square\ntype Id = i64\n",
        "File[TypeDecl[type Chord : NL > FieldDecl[root : TypeRef[i32] NL] FieldDecl[spread : TypeRef[i32] NL] <] TypeDecl[type Shape = SumType[TypeRef[Circle] | TypeRef[Square]] NL] TypeDecl[type Id = TypeRef[i64] NL]]",
    ),
    (
        "FieldDecl",
        "type Stack[T, const N]:\n    items: T[..N]\n    type: i32\n",
        "File[TypeDecl[type Stack GenericParams[[ GenericParam[T] , GenericParam[const N] ]] : NL > FieldDecl[items : TypeRef[T [ .. Size[NameRef[N]] ]] NL] FieldDecl[type : TypeRef[i32] NL] <]]",
    ),
    (
        "RecordLit",
        "Package {name: \"a\", version: \"1.0.0\"}\n",
        "File[PackageDecl[RecordLit[Package { Field[name : StringLit[\" a \"]] , Field[version : StringLit[\" 1.0.0 \"]] }] NL]]",
    ),
    (
        "Fields",
        "Package {}\nPackage {name: \"a\",}\n",
        "File[PackageDecl[RecordLit[Package { }] NL] PackageDecl[RecordLit[Package { Field[name : StringLit[\" a \"]] , }] NL]]",
    ),
    (
        "Result",
        "fn rise(cell x) -> bool: x.past(1) < x\n",
        "File[FnDecl[fn rise ParamList[( Param[cell x] )] FnResult[-> TypeRef[bool]] : FnBody[BinaryExpr[CallExpr[FieldExpr[NameRef[x] . past] ArgList[( Literal[1] )]] < NameRef[x]] NL]]]",
    ),
    (
        "Marker",
        "pattern P(const a, cell b, state c, stream d, channel e, out f, out state g, out stream h, out channel i):\n    Note\n",
        "File[PatternDecl[pattern P PatternParamList[( PatternParam[const a] , PatternParam[cell b] , PatternParam[state c] , PatternParam[stream d] , PatternParam[channel e] , PatternParam[out f] , PatternParam[out state g] , PatternParam[out stream h] , PatternParam[out channel i] )] : Block[NL > Placement[Note NL] <]]]",
    ),
    (
        "PatternItem",
        "pattern Voice(default root):\n    state gain(1.0)\n    let k = 2\n    stream s(0)\n    Note\n",
        "File[PatternDecl[pattern Voice PatternParamList[( PatternParam[default root] )] : Block[NL > CellDecl[state gain Init[( Literal[1.0] )] NL] LocalLet[let k = Literal[2] NL] CellDecl[stream s Init[( Literal[0] )] NL] Placement[Note NL] <]]]",
    ),
    (
        "PlacementType",
        "pattern Section(children notes: Note*, children links: Plays*) -> Clip:\n    Clip\n",
        "File[PatternDecl[pattern Section PatternParamList[( PatternParam[children notes : PlacementType[PlacementEntry[Note *]]] , PatternParam[children links : PlacementType[PlacementEntry[Plays *]]] )] -> PlacementType[PlacementEntry[Clip]] : Block[NL > Placement[Clip NL] <]]]",
    ),
    (
        "PlacementEntry",
        "pattern Stab(children chords: [Note+]*) -> (Note, [Note{2+}], node{..4}, wire{2..4}, lib::Kit+):\n    Note\n",
        "File[PatternDecl[pattern Stab PatternParamList[( PatternParam[children chords : PlacementType[PlacementEntry[[ PlacementEntry[Note +] ] *]]] )] -> PlacementType[( PlacementEntry[Note] , PlacementEntry[[ PlacementEntry[Note Count[{ 2 + }]] ]] , PlacementEntry[node Count[{ .. 4 }]] , PlacementEntry[wire Count[{ 2 .. 4 }]] , PlacementEntry[QualifiedName[lib :: Kit] +] )] : Block[NL > Placement[Note NL] <]]]",
    ),
    (
        "Count",
        "pattern P() -> (Note+, Note*, Note{3}, Note{2+}, Note{..4}, Note{2..4}):\n    Note\n",
        "File[PatternDecl[pattern P PatternParamList[( )] -> PlacementType[( PlacementEntry[Note +] , PlacementEntry[Note *] , PlacementEntry[Note Count[{ 3 }]] , PlacementEntry[Note Count[{ 2 + }]] , PlacementEntry[Note Count[{ .. 4 }]] , PlacementEntry[Note Count[{ 2 .. 4 }]] )] : Block[NL > Placement[Note NL] <]]]",
    ),
    (
        "GenericParams",
        "fn push[T, const N: i32](s: Stack[T, N], x: T) -> Stack[T, N]: s\n",
        "File[FnDecl[fn push GenericParams[[ GenericParam[T] , GenericParam[const N : TypeRef[i32]] ]] ParamList[( Param[s : TypeRef[Stack TypeArgs[[ TypeRef[T] , TypeRef[N] ]]]] , Param[x : TypeRef[T]] )] FnResult[-> TypeRef[Stack TypeArgs[[ TypeRef[T] , TypeRef[N] ]]]] : FnBody[NameRef[s] NL]]]",
    ),
    (
        "GenericParam",
        "type Pair[A, B: i32 | f32, const N, const M: i64] = A\n",
        "File[TypeDecl[type Pair GenericParams[[ GenericParam[A] , GenericParam[B : SumType[TypeRef[i32] | TypeRef[f32]]] , GenericParam[const N] , GenericParam[const M : TypeRef[i64]] ]] = TypeRef[A] NL]]",
    ),
    (
        "TypeArgs",
        "let a: Stack[i32, 8][4] = 1\nlet c: Box[str[..4]] = 1\n",
        "File[LetDecl[let a : TypeRef[Stack TypeArgs[[ TypeRef[i32] , Literal[8] ]] [ 4 ]] = Literal[1] NL] LetDecl[let c : TypeRef[Box TypeArgs[[ TypeRef[StrType[str [ .. 4 ]]] ]]] = Literal[1] NL]]",
    ),
    (
        "TypeArg",
        "let d: Stack[i32, 4 * 2] = 1\nlet e: Map[{i32: i32}[..2], N] = 1\n",
        "File[LetDecl[let d : TypeRef[Stack TypeArgs[[ TypeRef[i32] , BinaryExpr[Literal[4] * Literal[2]] ]]] = Literal[1] NL] LetDecl[let e : TypeRef[Map TypeArgs[[ TypeRef[MapType[{ TypeRef[i32] : TypeRef[i32] } [ .. 2 ]]] , TypeRef[N] ]]] = Literal[1] NL]]",
    ),
    (
        "PackageFile",
        "Package {\n    name: \"scales\",\n    version: \"2.4.1\",\n    license: \"MIT\",\n    authors: [\"Ada Lovelace <ada@example.com>\"],\n}\n\nrequire \"github.com/acme/util@1.2.0\"\nuse util as u\n\npublish major, minor\n",
        "File[PackageDecl[RecordLit[Package { Field[name : StringLit[\" scales \"]] , Field[version : StringLit[\" 2.4.1 \"]] , Field[license : StringLit[\" MIT \"]] , Field[authors : ArrayLit[[ StringLit[\" Ada Lovelace <ada@example.com> \"] ]]] , }] NL] Require[require StringLit[\" github.com/acme/util@1.2.0 \"] NL] Use[use util as u NL] Publish[publish PublishName[major] , PublishName[minor] NL]]",
    ),
    (
        "PackageItem",
        "Package {name: \"a\"}\nrequire \"../util\"\nuse util\npublish a\n",
        "File[PackageDecl[RecordLit[Package { Field[name : StringLit[\" a \"]] }] NL] Require[require StringLit[\" ../util \"] NL] Use[use util NL] Publish[publish PublishName[a] NL]]",
    ),
    (
        "PackageDecl",
        "Package {\n    name: \"scales\",\n    version: \"2.4.1\",\n}\n",
        "File[PackageDecl[RecordLit[Package { Field[name : StringLit[\" scales \"]] , Field[version : StringLit[\" 2.4.1 \"]] , }] NL]]",
    ),
    (
        "Require",
        "require \"../util\" as u\nrequire \"github.com/acme/util@1.2.0\"\n",
        "File[Require[require StringLit[\" ../util \"] as u NL] Require[require StringLit[\" github.com/acme/util@1.2.0 \"] NL]]",
    ),
    (
        "QualifiedName",
        "let b: lib::Chord = 1\npublish scales::minor as Minor\n",
        "File[LetDecl[let b : TypeRef[QualifiedName[lib :: Chord]] = Literal[1] NL] Publish[publish PublishName[QualifiedName[scales :: minor] as Minor] NL]]",
    ),
    (
        "Use",
        "use drums\nuse drums: Kit, Snare\nuse drums as d\n",
        "File[Use[use drums NL] Use[use drums : UseName[Kit] , UseName[Snare] NL] Use[use drums as d NL]]",
    ),
    (
        "UseName",
        "use drums: Kit as Set, Snare\n",
        "File[Use[use drums : UseName[Kit as Set] , UseName[Snare] NL]]",
    ),
    (
        "Publish",
        "publish Drum as Kick, Kit, Hit\npublish major\n",
        "File[Publish[publish PublishName[Drum as Kick] , PublishName[Kit] , PublishName[Hit] NL] Publish[publish PublishName[major] NL]]",
    ),
    (
        "PublishName",
        "publish major, Drum as Kick, scales::minor as Minor\n",
        "File[Publish[publish PublishName[major] , PublishName[Drum as Kick] , PublishName[QualifiedName[scales :: minor] as Minor] NL]]",
    ),
    (
        "Fallback",
        "let a = port ?? 80\nlet c = x ?? y ?? z\nlet b = x ?? base + 1\nlet d = port ?? 80 > 1024\n",
        "File[LetDecl[let a = BinaryExpr[NameRef[port] ?? Literal[80]] NL] LetDecl[let c = BinaryExpr[NameRef[x] ?? BinaryExpr[NameRef[y] ?? NameRef[z]]] NL] LetDecl[let b = BinaryExpr[NameRef[x] ?? BinaryExpr[NameRef[base] + Literal[1]]] NL] LetDecl[let d = BinaryExpr[BinaryExpr[NameRef[port] ?? Literal[80]] > Literal[1024]] NL]]",
    ),
    (
        "AltType",
        "let y = a is not i32[2]\nlet w = a is str\nlet m = match s:\n    none: 2\n    i32: 3\n    lib::T: 6\n    Box[i32]: 7\n    else: 8\n",
        "File[LetDecl[let y = BinaryExpr[NameRef[a] is not AltType[TypeRef[i32 [ 2 ]]]] NL] LetDecl[let w = BinaryExpr[NameRef[a] is AltType[TypeRef[StrType[str]]]] NL] LetDecl[let m = MatchBlock[match NameRef[s] : NL > MatchArm[AltType[TypeRef[none]] : Literal[2] NL] MatchArm[AltType[TypeRef[i32]] : Literal[3] NL] MatchArm[AltType[TypeRef[QualifiedName[lib :: T]]] : Literal[6] NL] MatchArm[AltType[TypeRef[Box TypeArgs[[ TypeRef[i32] ]]]] : Literal[7] NL] ElseArm[else : Literal[8] NL] <]]]",
    ),
    (
        "Arg",
        "let a = count(held, p: p > 60) + f(x) + g(k=(a, b): a + b)\n",
        "File[LetDecl[let a = BinaryExpr[BinaryExpr[CallExpr[NameRef[count] ArgList[( NameRef[held] , Lambda[p : BinaryExpr[NameRef[p] > Literal[60]]] )]] + CallExpr[NameRef[f] ArgList[( NameRef[x] )]]] + CallExpr[NameRef[g] ArgList[( NamedArg[k = Lambda[( LambdaParams[LambdaParam[a] , LambdaParam[b]] ) : BinaryExpr[NameRef[a] + NameRef[b]]]] )]]] NL]]",
    ),
    (
        "Lambda",
        "let inc = x: x + 1\nlet a = count(held, (p: i32) -> bool: p < 48)\nlet z = f((): 1)\n",
        "File[LetDecl[let inc = Lambda[x : BinaryExpr[NameRef[x] + Literal[1]]] NL] LetDecl[let a = CallExpr[NameRef[count] ArgList[( NameRef[held] , Lambda[( LambdaParams[LambdaParam[p : TypeRef[i32]]] ) FnResult[-> TypeRef[bool]] : BinaryExpr[NameRef[p] < Literal[48]]] )]] NL] LetDecl[let z = CallExpr[NameRef[f] ArgList[( Lambda[( ) : Literal[1]] )]] NL]]",
    ),
    (
        "LambdaParams",
        "let add = (a, b): a + b\n",
        "File[LetDecl[let add = Lambda[( LambdaParams[LambdaParam[a] , LambdaParam[b]] ) : BinaryExpr[NameRef[a] + NameRef[b]]] NL]]",
    ),
    (
        "LambdaParam",
        "let add = (a: i32, b): a + b\n",
        "File[LetDecl[let add = Lambda[( LambdaParams[LambdaParam[a : TypeRef[i32]] , LambdaParam[b]] ) : BinaryExpr[NameRef[a] + NameRef[b]]] NL]]",
    ),
    (
        "MapLit",
        "let m = {}\nlet n = {\"a\": 1, \"b\": 2,}\n",
        "File[LetDecl[let m = MapLit[{ }] NL] LetDecl[let n = MapLit[{ MapEntry[StringLit[\" a \"] : Literal[1]] , MapEntry[StringLit[\" b \"] : Literal[2]] , }] NL]]",
    ),
    (
        "Entry",
        "let m = {\"k\": v, k2: v + 1, Saw: 2, 3: x}\n",
        "File[LetDecl[let m = MapLit[{ MapEntry[StringLit[\" k \"] : NameRef[v]] , MapEntry[NameRef[k2] : BinaryExpr[NameRef[v] + Literal[1]]] , MapEntry[NameRef[Saw] : Literal[2]] , MapEntry[Literal[3] : NameRef[x]] }] NL]]",
    ),
];

#[test]
fn every_production_case_parses_to_its_tree() {
    for (production, text, expected) in CASES {
        assert_eq!(support::clean_tree(text), *expected, "{production}");
        assert_eq!(
            support::parsed(text).root().to_string(),
            *text,
            "{production}"
        );
    }
}

fn pending_list_problems(
    productions: &[String],
    cases: &[&str],
    list: &Path,
    write: bool,
) -> Vec<String> {
    let has_case = |name: &str| cases.contains(&name);
    let listed = pending_lines(list);
    let names: BTreeSet<&String> = productions.iter().collect();
    let uncased_unlisted = productions
        .iter()
        .filter(|name| !listed.contains(*name) && !has_case(name))
        .map(|name| {
            format!(
                "{name} has no parse case: add the line `{name}` to {}",
                list.display()
            )
        });
    if write {
        let text = std::fs::read_to_string(list)
            .unwrap_or_else(|error| panic!("{} reads: {error}", list.display()));
        let mut kept = 0;
        let mut written = String::new();
        for line in text.lines() {
            let name = line.trim();
            let document = name.is_empty() || name.starts_with('#');
            let pending = names.contains(&name.to_string()) && !has_case(name);
            if document || pending {
                kept += usize::from(pending);
                written.push_str(line);
                written.push('\n');
            }
        }
        std::fs::write(list, written)
            .unwrap_or_else(|error| panic!("{} writes: {error}", list.display()));
        let mut problems = vec![format!(
            "wrote {} with {kept} lines; run again without SYMPHONY_PENDING_PRODUCTIONS to check it",
            list.display()
        )];
        problems.extend(uncased_unlisted);
        return problems;
    }
    let mut problems: Vec<String> = productions
        .iter()
        .filter(|name| listed.contains(*name) && has_case(name))
        .map(|name| {
            format!(
                "{name} has a parse case: remove the line `{name}` from {}",
                list.display()
            )
        })
        .collect();
    problems.extend(uncased_unlisted);
    problems.extend(
        listed
            .iter()
            .filter(|line| !names.contains(line))
            .map(|line| {
                format!(
                    "no production is `{line}`: remove the line from {}",
                    list.display()
                )
            }),
    );
    problems
}

#[test]
fn the_pending_list_names_exactly_the_productions_of_the_spec_without_a_case() {
    let write = std::env::var("SYMPHONY_PENDING_PRODUCTIONS").is_ok_and(|mode| mode == "write");
    let cases: Vec<&str> = CASES.iter().map(|(production, _, _)| *production).collect();
    let problems = pending_list_problems(
        &spec_productions(),
        &cases,
        Path::new(PENDING_PRODUCTIONS),
        write,
    );
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_case_names_a_production_of_the_spec() {
    let productions = spec_productions();
    let unknown: Vec<&str> = CASES
        .iter()
        .map(|(production, _, _)| *production)
        .filter(|name| !productions.iter().any(|production| production == name))
        .collect();
    assert!(
        unknown.is_empty(),
        "cases naming no production: {unknown:?}"
    );
}

const SMALL_PRODUCTIONS: [&str; 3] = ["Song", "Verse", "Chorus"];

const SMALL_CASES: [&str; 1] = ["Song"];

fn small_problems(list: &Path, write: bool) -> Vec<String> {
    let productions: Vec<String> = SMALL_PRODUCTIONS
        .iter()
        .map(|name| name.to_string())
        .collect();
    pending_list_problems(&productions, &SMALL_CASES, list, write)
}

fn list_file(name: &str, text: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&path, text).expect("the list writes");
    path
}

#[test]
fn a_listed_production_with_a_case_fails_naming_its_line() {
    let list = list_file(
        "pending-productions-listed-cased.txt",
        "# pending\n\nSong\nVerse\nChorus\n",
    );
    assert_eq!(
        small_problems(&list, false),
        [format!(
            "Song has a parse case: remove the line `Song` from {}",
            list.display()
        )]
    );
}

#[test]
fn an_unlisted_production_without_a_case_fails_naming_its_line() {
    let list = list_file("pending-productions-unlisted-uncased.txt", "Verse\n");
    assert_eq!(
        small_problems(&list, false),
        [format!(
            "Chorus has no parse case: add the line `Chorus` to {}",
            list.display()
        )]
    );
}

#[test]
fn a_listed_line_that_names_no_production_fails_naming_it() {
    let list = list_file(
        "pending-productions-no-production.txt",
        "Verse\nChorus\nBridge\n",
    );
    assert_eq!(
        small_problems(&list, false),
        [format!(
            "no production is `Bridge`: remove the line from {}",
            list.display()
        )]
    );
}

#[test]
fn a_list_that_names_exactly_the_productions_without_a_case_passes() {
    let list = list_file("pending-productions-exact.txt", "Chorus\nVerse\n");
    assert_eq!(small_problems(&list, false), Vec::<String>::new());
}

#[test]
fn the_write_mode_removes_the_lines_of_productions_with_a_case_and_fails_on_a_production_without_one_296()
 {
    let list = list_file(
        "pending-productions-write.txt",
        "# pending\nSong\nVerse\nBridge\n",
    );
    assert_eq!(
        small_problems(&list, true),
        [
            format!(
                "wrote {} with 1 lines; run again without SYMPHONY_PENDING_PRODUCTIONS to check it",
                list.display()
            ),
            format!(
                "Chorus has no parse case: add the line `Chorus` to {}",
                list.display()
            ),
        ]
    );
    assert_eq!(
        std::fs::read_to_string(&list).expect("the list reads"),
        "# pending\nVerse\n"
    );
    assert_eq!(
        small_problems(&list, false),
        [format!(
            "Chorus has no parse case: add the line `Chorus` to {}",
            list.display()
        )]
    );
}

#[test]
fn the_write_mode_keeps_a_list_that_names_every_production_without_a_case_296() {
    let list = list_file(
        "pending-productions-write-exact.txt",
        "Chorus\nSong\nVerse\n",
    );
    assert_eq!(
        small_problems(&list, true),
        [format!(
            "wrote {} with 2 lines; run again without SYMPHONY_PENDING_PRODUCTIONS to check it",
            list.display()
        )]
    );
    assert_eq!(
        std::fs::read_to_string(&list).expect("the list reads"),
        "Chorus\nVerse\n"
    );
    assert_eq!(small_problems(&list, false), Vec::<String>::new());
}
