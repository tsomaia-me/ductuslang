mod support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use symphonyscript_fixtures::{PENDING_PRODUCTIONS, pending_lines, spec_productions};

const CASES: &[(&str, &str, &str)] = &[
    (
        "File",
        "input a = 1\nClip c\n",
        "File[InputDecl[input a = Literal[1] NL] Placement[Clip c NL]]",
    ),
    (
        "TopItem",
        "input a = 1\nlet b = 2\nstate s = 3\nfn f(x): x\npattern P():\n    Note\nClip c\n",
        "File[InputDecl[input a = Literal[1] NL] LetDecl[let b = Literal[2] NL] StateDecl[state s = Literal[3] NL] FnDecl[fn f ParamList[( Param[x] )] : FnBody[NameRef[x] NL]] PatternDecl[pattern P PatternParamList[( )] : Block[NL > Placement[Note NL] <]] Placement[Clip c NL]]",
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
        "input a: f32[2][3] = 0\ninput w: Wave = Wave::Saw\ninput h: u8[2][..4] = []\n",
        "File[InputDecl[input a : TypeRef[f32 [ 2 ] [ 3 ]] = Literal[0] NL] InputDecl[input w : TypeRef[Wave] = VariantRef[Wave :: Saw] NL] InputDecl[input h : TypeRef[u8 [ 2 ] [ .. 4 ]] = ArrayLit[[ ]] NL]]",
    ),
    (
        "ScalarType",
        "input z: u8 = 0\ninput a: i32 = 0\ninput b: i64 = 0\ninput c: f32 = 0\ninput d: f64 = 0\ninput e: bool = true\n",
        "File[InputDecl[input z : TypeRef[u8] = Literal[0] NL] InputDecl[input a : TypeRef[i32] = Literal[0] NL] InputDecl[input b : TypeRef[i64] = Literal[0] NL] InputDecl[input c : TypeRef[f32] = Literal[0] NL] InputDecl[input d : TypeRef[f64] = Literal[0] NL] InputDecl[input e : TypeRef[bool] = Literal[true] NL]]",
    ),
    (
        "Expr",
        "let a = if c then 1 else 2\nlet b = x or y\n",
        "File[LetDecl[let a = IfExpr[if NameRef[c] then Literal[1] else Literal[2]] NL] LetDecl[let b = BinaryExpr[NameRef[x] or NameRef[y]] NL]]",
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
        "InputDecl",
        "input cutoff = 0.5\ninput fine: f32 = 0\n",
        "File[InputDecl[input cutoff = Literal[0.5] NL] InputDecl[input fine : TypeRef[f32] = Literal[0] NL]]",
    ),
    (
        "LetDecl",
        "let a: i32 = 1\nlet b = match a:\n    0: 1\n    else: 2\n",
        "File[LetDecl[let a : TypeRef[i32] = Literal[1] NL] LetDecl[let b = MatchBlock[match NameRef[a] : NL > MatchArm[Literal[0] : Literal[1] NL] ElseArm[else : Literal[2] NL] <]]]",
    ),
    (
        "FnDecl",
        "fn f(): 1\nfn g(x):\n    x\n",
        "File[FnDecl[fn f ParamList[( )] : FnBody[Literal[1] NL]] FnDecl[fn g ParamList[( Param[x] )] : FnBody[NL > NameRef[x] NL <]]]",
    ),
    (
        "Bounded",
        "fn f(n) bounded 10 else 0: f(n + 1)\n",
        "File[FnDecl[fn f ParamList[( Param[n] )] Bounded[bounded 10 else Literal[0]] : FnBody[CallExpr[NameRef[f] ArgList[( BinaryExpr[NameRef[n] + Literal[1]] )]] NL]]]",
    ),
    (
        "StateDecl",
        "state[2] s: f32 = s{2, 0.0}\nstate m = match k:\n    0: 1\n    else: 2\n",
        "File[StateDecl[state [ 2 ] s : TypeRef[f32] = History[s { Literal[2] , Literal[0.0] }] NL] StateDecl[state m = MatchBlock[match NameRef[k] : NL > MatchArm[Literal[0] : Literal[1] NL] ElseArm[else : Literal[2] NL] <]]]",
    ),
    (
        "History",
        "state s = s{1, 0} + t{2}\n",
        "File[StateDecl[state s = BinaryExpr[History[s { Literal[1] , Literal[0] }] + History[t { Literal[2] }]] NL]]",
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
    if write {
        let uncased: Vec<&String> = productions.iter().filter(|name| !has_case(name)).collect();
        let text: String = uncased.iter().map(|name| format!("{name}\n")).collect();
        std::fs::write(list, text)
            .unwrap_or_else(|error| panic!("{} writes: {error}", list.display()));
        return vec![format!(
            "wrote {} with {} lines; run again without SYMPHONY_PENDING_PRODUCTIONS to check it",
            list.display(),
            uncased.len()
        )];
    }
    let listed = pending_lines(list);
    let mut problems: Vec<String> = productions
        .iter()
        .filter_map(|name| match (listed.contains(name), has_case(name)) {
            (true, true) => Some(format!(
                "{name} has a parse case: remove the line `{name}` from {}",
                list.display()
            )),
            (false, false) => Some(format!(
                "{name} has no parse case: add the line `{name}` to {}",
                list.display()
            )),
            _ => None,
        })
        .collect();
    let names: BTreeSet<&String> = productions.iter().collect();
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
        .filter(|name| !RETIRING_PRODUCTIONS.contains(name))
        .collect();
    assert!(
        unknown.is_empty(),
        "cases naming no production: {unknown:?}"
    );
}

const RETIRING_PRODUCTIONS: [&str; 1] = ["Bounded"];

#[test]
fn every_retiring_production_has_left_the_spec_and_keeps_its_case() {
    let productions = spec_productions();
    for name in RETIRING_PRODUCTIONS {
        assert!(
            !productions.iter().any(|production| production == name),
            "{name} is still a production of the spec"
        );
        assert!(
            CASES.iter().any(|(production, _, _)| *production == name),
            "{name} has no case"
        );
    }
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
fn the_write_mode_writes_the_productions_without_a_case_in_spec_order_and_fails() {
    let list = list_file("pending-productions-write.txt", "Song\n");
    assert_eq!(
        small_problems(&list, true),
        [format!(
            "wrote {} with 2 lines; run again without SYMPHONY_PENDING_PRODUCTIONS to check it",
            list.display()
        )]
    );
    assert_eq!(
        std::fs::read_to_string(&list).expect("the list reads"),
        "Verse\nChorus\n"
    );
    assert_eq!(small_problems(&list, false), Vec::<String>::new());
}
