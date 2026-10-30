mod support;

use symphonyscript::Code;
use symphonyscript::syntax::{SyntaxKind, lex};
use symphonyscript_fixtures::vocabulary;

type Expected = &'static [(Code, u32, u32)];

const BROKEN: &[(&str, Expected, &str)] = &[
    (
        "input = 5\nlet b = 2\n",
        &[(Code::InputSyntax, 6, 7)],
        "File[InputDecl[input = Literal[5] NL] LetDecl[let b = Literal[2] NL]]",
    ),
    (
        "Clip c:\n    Note |\n    Note/62\n",
        &[(Code::AttributeSyntax, 18, 19)],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note Attrs[|] NL] Placement[Note DefaultArg[/ Literal[62]] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/60 Note | velocity=1\n    Note/62\n",
        &[(Code::InlinePlacement, 25, 26)],
        "File[Placement[Clip c Body[: Block[NL > Line[Placement[Note DefaultArg[/ Literal[60]]] Placement[Note] ErrorNode[| velocity = 1] NL] Placement[Note DefaultArg[/ Literal[62]] NL] <]]]]",
    ),
    (
        "let a = 1 < 2 < 3\nlet b = 1\n",
        &[(Code::ChainedComparison, 14, 15)],
        "File[LetDecl[let a = BinaryExpr[BinaryExpr[Literal[1] < Literal[2]] < Literal[3]] NL] LetDecl[let b = Literal[1] NL]]",
    ),
    (
        "let a = 1 + if b then 1 else 2\n",
        &[(Code::UnparenthesizedIf, 12, 14)],
        "File[LetDecl[let a = BinaryExpr[Literal[1] + IfExpr[if NameRef[b] then Literal[1] else Literal[2]]] NL]]",
    ),
    (
        "let a = f(1, x=2)\n",
        &[(Code::MixedArguments, 13, 14)],
        "File[LetDecl[let a = CallExpr[NameRef[f] ArgList[( Literal[1] , NamedArg[x = Literal[2]] )]] NL]]",
    ),
    (
        "Plays: verse\nClip verse\n",
        &[(Code::TopLevelWire, 0, 5)],
        "File[WirePlacement[Plays : Destination[NameRef[verse] NL]] Placement[Clip verse NL]]",
    ),
    (
        "Clip c:\n    [Note Plays]\n",
        &[(Code::MisplacedWire, 18, 23)],
        "File[Placement[Clip c Body[: Block[NL > Line[Bundle[[ Placement[Note] Placement[Plays] ]] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note'x\n",
        &[(Code::FlagSyntax, 17, 18)],
        "File[Placement[Clip c Body[: Block[NL > Line[Placement[Note Flags[']] Placement[x] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/\n    Note/1\n",
        &[(Code::DefaultArgSyntax, 17, 18)],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/] NL] Placement[Note DefaultArg[/ Literal[1]] NL] <]]]]",
    ),
    (
        "Clip c:\n    []\n",
        &[(Code::BundleSyntax, 13, 14)],
        "File[Placement[Clip c Body[: Block[NL > Line[Bundle[[ ]] NL] <]]]]",
    ),
    (
        "Clip c:\n    for in 0..3:\n        Note\n",
        &[(Code::ForSyntax, 16, 18)],
        "File[Placement[Clip c Body[: Block[NL > ForBlock[for in RangeExpr[Literal[0] .. Literal[3]] : Block[NL > Placement[Note NL] <]] <]]]]",
    ),
    (
        "Clip c:\n    when x\n        Note\n    Note\n",
        &[(Code::WhenSyntax, 18, 19)],
        "File[Placement[Clip c Body[: Block[NL > WhenBlock[when NameRef[x] NL] ErrorNode[> Note NL <] Placement[Note NL] <]]]]",
    ),
    (
        "let a = match b:\n    1\n    else: 2\nlet c = 3\n",
        &[(Code::MatchSyntax, 22, 23)],
        "File[LetDecl[let a = MatchBlock[match NameRef[b] : NL > MatchArm[Literal[1] NL] ElseArm[else : Literal[2] NL] <]] LetDecl[let c = Literal[3] NL]]",
    ),
    (
        "let x = [1, 2\nlet y = 3\n",
        &[(Code::ExpressionSyntax, 14, 17)],
        "File[LetDecl[let x = ArrayLit[[ Literal[1] , Literal[2]] NL] LetDecl[let y = Literal[3] NL]]",
    ),
    (
        "fn f(x:\n    x\nlet y = 1\n",
        &[(Code::FnSyntax, 6, 7)],
        "File[FnDecl[fn f ParamList[( Param[x]] : FnBody[NameRef[x] NL]] LetDecl[let y = Literal[1] NL]]",
    ),
    (
        "pattern P():\n    let a = 1\nClip c\n",
        &[(Code::PlacementSyntax, 27, 27)],
        "File[PatternDecl[pattern P PatternParamList[( )] : Block[NL > LocalLet[let a = Literal[1] NL] <]] Placement[Clip c NL]]",
    ),
    (
        "Clip c:\n    Note\n  Note\n",
        &[(Code::DedentMismatch, 19, 20)],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note NL] <]]] Placement[Note NL]]",
    ),
    (
        "input a = 1;\n",
        &[(Code::Semicolon, 11, 12)],
        "File[InputDecl[input a = Literal[1] ErrorNode[;] NL]]",
    ),
    (
        "input a = 1\n    input b = 2\nlet c = 3\n",
        &[(Code::UnexpectedIndent, 16, 21)],
        "File[InputDecl[input a = Literal[1] NL] ErrorNode[> input b = 2 NL <] LetDecl[let c = Literal[3] NL]]",
    ),
    (
        "Clip c | :\n    Note\n",
        &[(Code::AttributeSyntax, 9, 10)],
        "File[Placement[Clip c Attrs[|] Body[: Block[NL > Placement[Note NL] <]]]]",
    ),
    (
        "let a = match b\n",
        &[(Code::MatchSyntax, 15, 16)],
        "File[LetDecl[let a = MatchBlock[match NameRef[b] NL]]]",
    ),
    (
        "Clip c:\n    Note | velocity=1 when x\n",
        &[(Code::PlacementSyntax, 30, 34)],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note Attrs[| AttrEntry[velocity = Literal[1]]] ErrorNode[when x] NL] <]]]]",
    ),
    (
        "let a = \"x{1 2}y\"\nlet b = 1\n",
        &[(Code::ExpressionSyntax, 13, 14)],
        "File[LetDecl[let a = StringLit[\" x Interpolation[{ Literal[1] ErrorNode[2] }] y \"] NL] LetDecl[let b = Literal[1] NL]]",
    ),
    (
        "Clip\n",
        &[(Code::PlacementSyntax, 4, 5)],
        "File[Placement[Clip NL]]",
    ),
    (
        "let a = )\nlet b = 1\n",
        &[(Code::ExpressionSyntax, 8, 9)],
        "File[LetDecl[let a = ErrorNode[)] NL] LetDecl[let b = Literal[1] NL]]",
    ),
    (
        "Clip c:\n    let a = 1\n    Note\n",
        &[(Code::LetSyntax, 12, 15)],
        "File[Placement[Clip c Body[: Block[NL > LocalLet[let a = Literal[1] NL] Placement[Note NL] <]]]]",
    ),
    (
        "fn f(): 1\n    2\n",
        &[(Code::UnexpectedIndent, 14, 15)],
        "File[FnDecl[fn f ParamList[( )] : FnBody[Literal[1] NL]] ErrorNode[> 2 NL <]]",
    ),
    (
        "Clip c:\n    Note | !velocity=1\n",
        &[(Code::AttributeSyntax, 28, 29)],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note Attrs[| AttrEntry[! velocity]] ErrorNode[= 1] NL] <]]]]",
    ),
    (
        "input a: i32[x] = 0\n",
        &[(Code::TypeSyntax, 13, 14)],
        "File[InputDecl[input a : TypeRef[i32 [] NameRef[x] ErrorNode[] = 0] NL]]",
    ),
    (
        "let a = i32 x\n",
        &[(Code::ConversionSyntax, 12, 13)],
        "File[LetDecl[let a = Conversion[i32] ErrorNode[x] NL]]",
    ),
    (
        "Clip c:\n    when:\n        else: Note\n        a: Note\n",
        &[(Code::WhenSyntax, 45, 46)],
        "File[Placement[Clip c Body[: Block[NL > WhenBlock[when : NL > WhenElse[else : Block[Members[Placement[Note]] NL]] GuardArm[NameRef[a] : Block[Members[Placement[Note]] NL]] <] <]]]]",
    ),
    (
        "let a = [ , ]\n",
        &[(Code::ExpressionSyntax, 10, 11)],
        "File[LetDecl[let a = ArrayLit[[ , ]] NL]]",
    ),
    (
        "Clip c: 5\n",
        &[(Code::PlacementSyntax, 8, 9)],
        "File[Placement[Clip c Body[: Block[Members[] ErrorNode[5] NL]]]]",
    ),
    (
        "Clip c:\n    (Plays: c)\n",
        &[(Code::MisplacedWire, 13, 18)],
        "File[Placement[Clip c Body[: Block[NL > Line[ParenPlacement[( Placement[Plays]] ErrorNode[: c )] NL] <]]]]",
    ),
    (
        "let a = match b:\n    else: 1\n    0: 2\n",
        &[(Code::MatchSyntax, 33, 34)],
        "File[LetDecl[let a = MatchBlock[match NameRef[b] : NL > ElseArm[else : Literal[1] NL] MatchArm[Literal[0] : Literal[2] NL] <]]]",
    ),
    (
        "let a = [1, @, 2]\n",
        &[(Code::ExpressionSyntax, 12, 13)],
        "File[LetDecl[let a = ArrayLit[[ Literal[1] ,] ErrorNode[@ , 2 ]] NL]]",
    ),
    (
        "Clip c:\n    [Note [Note]]\n",
        &[(Code::BundleSyntax, 18, 19)],
        "File[Placement[Clip c Body[: Block[NL > Line[Bundle[[ Placement[Note] Bundle[[ Placement[Note] ]] ]] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note\n    Note:\n",
        &[(Code::PlacementSyntax, 27, 27)],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note NL] Placement[Note Body[: Block[NL]]] <]]]]",
    ),
    (
        "Clip c:\n    Note'as hook\n",
        &[(Code::FlagSyntax, 17, 19)],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note Flags['] as hook NL] <]]]]",
    ),
    (
        "Clip c:\n    Note'when x\n",
        &[(Code::FlagSyntax, 17, 21)],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note Flags['] Gate[when NameRef[x]] NL] <]]]]",
    ),
    (
        "let a = (1 +\nlet b = )\n",
        &[
            (Code::ExpressionSyntax, 13, 16),
            (Code::ExpressionSyntax, 21, 22),
        ],
        "File[LetDecl[let a = ParenExpr[( BinaryExpr[Literal[1] +]] NL] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "let x = [1, 2\nlet y = )\n",
        &[
            (Code::ExpressionSyntax, 14, 17),
            (Code::ExpressionSyntax, 22, 23),
        ],
        "File[LetDecl[let x = ArrayLit[[ Literal[1] , Literal[2]] NL] LetDecl[let y = ErrorNode[)] NL]]",
    ),
    (
        "let a = f(\"abc\nlet b = )\n",
        &[
            (Code::UnterminatedString, 10, 14),
            (Code::ExpressionSyntax, 23, 24),
        ],
        "File[LetDecl[let a = CallExpr[NameRef[f] ArgList[( StringLit[\" abc]]] NL] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "Clip c:\n    Note/(1 +\nlet z = )\n",
        &[
            (Code::ExpressionSyntax, 22, 25),
            (Code::ExpressionSyntax, 30, 31),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( BinaryExpr[Literal[1] +]]] NL] <]]] LetDecl[let z = ErrorNode[)] NL]]",
    ),
    (
        "let b = x.0\n",
        &[(Code::FloatDigits, 9, 11)],
        "File[LetDecl[let b = NameRef[x] ErrorNode[.0] NL]]",
    ),
    (
        "Clip 音楽 | gain=1\n            level=2\n",
        &[(Code::ClauseColumn, 33, 34)],
        "File[Placement[Clip 音楽 Attrs[| AttrEntry[gain = Literal[1]] AttrEntry[level = Literal[2]]] NL]]",
    ),
    (
        "Clip c:\n    Note/a[0]\n",
        &[(Code::DefaultArgSyntax, 18, 19)],
        "File[Placement[Clip c Body[: Block[NL > Line[Placement[Note DefaultArg[/ NameRef[a]]] Bundle[[] ErrorNode[0 ]] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/f(1)\n",
        &[(Code::DefaultArgSyntax, 18, 19)],
        "File[Placement[Clip c Body[: Block[NL > Line[Placement[Note DefaultArg[/ NameRef[f]]] ParenPlacement[(] ErrorNode[1 )] NL] <]]]]",
    ),
    (
        "input a 1_\nlet b = 2\n",
        &[
            (Code::InputSyntax, 8, 10),
            (Code::MisplacedUnderscore, 9, 10),
        ],
        "File[InputDecl[input a Literal[1_] NL] LetDecl[let b = Literal[2] NL]]",
    ),
    (
        "let a = [1 @\n2, 3]\nlet b = 4\n",
        &[(Code::ExpressionSyntax, 11, 12)],
        "File[LetDecl[let a = ArrayLit[[ Literal[1]] ErrorNode[@ 2 , 3 ]] NL] LetDecl[let b = Literal[4] NL]]",
    ),
    (
        "Clip c:\n    Note/(1 +\n    when x: Note\n",
        &[(Code::ExpressionSyntax, 26, 30)],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( BinaryExpr[Literal[1] +]]] NL] WhenBlock[when NameRef[x] : Block[Members[Placement[Note]] NL]] <]]]]",
    ),
    (
        "let a = 1_ 2\n",
        &[
            (Code::MisplacedUnderscore, 9, 10),
            (Code::LetSyntax, 11, 12),
        ],
        "File[LetDecl[let a = Literal[1_] ErrorNode[2] NL]]",
    ),
    (
        "let a = 0b12 3\n",
        &[(Code::PrefixedSuffix, 11, 12), (Code::LetSyntax, 13, 14)],
        "File[LetDecl[let a = Literal[0b12] ErrorNode[3] NL]]",
    ),
    (
        "let a = f(1 2,\nb)\nlet c = 3\n",
        &[(Code::ExpressionSyntax, 12, 13)],
        "File[LetDecl[let a = CallExpr[NameRef[f] ArgList[( Literal[1]]] ErrorNode[2 , b )] NL] LetDecl[let c = Literal[3] NL]]",
    ),
    (
        "let a = (1 2\nx)\nlet b = )\n",
        &[
            (Code::ExpressionSyntax, 11, 12),
            (Code::ExpressionSyntax, 24, 25),
        ],
        "File[LetDecl[let a = ParenExpr[( Literal[1]] ErrorNode[2 x )] NL] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "Clip c:\n    Note/a[0\n    Note/1\nlet b = )\n",
        &[
            (Code::DefaultArgSyntax, 18, 19),
            (Code::ExpressionSyntax, 40, 41),
        ],
        "File[Placement[Clip c Body[: Block[NL > Line[Placement[Note DefaultArg[/ NameRef[a]]] Bundle[[] ErrorNode[0] NL] Placement[Note DefaultArg[/ Literal[1]] NL] <]]] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "Clip c:\n    Note/f(1\n    Note/1\nlet b = )\n",
        &[
            (Code::DefaultArgSyntax, 18, 19),
            (Code::ExpressionSyntax, 40, 41),
        ],
        "File[Placement[Clip c Body[: Block[NL > Line[Placement[Note DefaultArg[/ NameRef[f]]] ParenPlacement[(] ErrorNode[1] NL] Placement[Note DefaultArg[/ Literal[1]] NL] <]]] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    Note as hook /3\n    Note'/4\n    Note\n    Plays: c\nlet b = )\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::ExpressionSyntax, 84, 85),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Placement[Note as hook DefaultArg[/ Literal[3]] NL] Placement[Note Flags['] DefaultArg[/ Literal[4]] NL] Placement[Note NL] WirePlacement[Plays : Destination[NameRef[c] NL]] <]]] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    Plays as w: d\n",
        &[(Code::DefaultArgSyntax, 20, 21)],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] WirePlacement[Plays as w : Destination[NameRef[d] NL]] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2 +\n    b / 2)\n    Note/3\n",
        &[(Code::DefaultArgSyntax, 20, 21)],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2 + b / 2 )] NL] Placement[Note DefaultArg[/ Literal[3]] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a / 2)\nlet b = )\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::ExpressionSyntax, 41, 42),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2 a / 2 )] NL] <]]] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a/2)\nlet b = )\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::ExpressionSyntax, 39, 40),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2 a / 2 )] NL] <]]] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a/(2)\n",
        &[(Code::DefaultArgSyntax, 20, 21)],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Placement[a DefaultArg[/ ParenExpr[( Literal[2] )]] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    Note /3\n    Note | velocity=1\n",
        &[(Code::DefaultArgSyntax, 20, 21)],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Placement[Note DefaultArg[/ Literal[3]] NL] Placement[Note Attrs[| AttrEntry[velocity = Literal[1]]] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/[1 2,\n    a/2,\n    b]\n",
        &[(Code::DefaultArgSyntax, 17, 18)],
        "File[Placement[Clip c Body[: Block[NL > Line[Placement[Note DefaultArg[/]] Bundle[[] ErrorNode[1 2 , a / 2 , b ]] NL] <]]]]",
    ),
    (
        "let a = [\nClip c:\n    Note\nClip d:\n    Note ^\n",
        &[
            (Code::ExpressionSyntax, 10, 14),
            (Code::MisplacedCaret, 44, 45),
        ],
        "File[LetDecl[let a = ArrayLit[[] NL] Placement[Clip c Body[: Block[NL > Placement[Note NL] <]]] Placement[Clip d Body[: Block[NL > Placement[Note ErrorNode[^] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a/b when c)\nlet b = )\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::ExpressionSyntax, 46, 47),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2 a / b when c )] NL] <]]] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "Clip c:\n    [Note/60\nClip d:\n    Note/1\nClip e:\n    Note ^\n",
        &[(Code::BundleSyntax, 21, 25), (Code::MisplacedCaret, 57, 58)],
        "File[Placement[Clip c Body[: Block[NL > Line[Bundle[[ Placement[Note DefaultArg[/ Literal[60]]]] NL] <]]] Placement[Clip d Body[: Block[NL > Placement[Note DefaultArg[/ Literal[1]] NL] <]]] Placement[Clip e Body[: Block[NL > Placement[Note ErrorNode[^] NL] <]]]]",
    ),
    (
        "Clip c:\n    (\n    Plays | gain=1)\n",
        &[(Code::MisplacedWire, 18, 23), (Code::WireSyntax, 32, 33)],
        "File[Placement[Clip c Body[: Block[NL > Line[ParenPlacement[( Placement[]] NL] WirePlacement[Plays Attrs[| AttrEntry[gain = Literal[1]]] ErrorNode[)] NL] <]]]]",
    ),
    (
        "Clip c:\n    [Note\n    [Note]]\n",
        &[
            (Code::BundleSyntax, 22, 23),
            (Code::InlinePlacement, 28, 29),
        ],
        "File[Placement[Clip c Body[: Block[NL > Line[Bundle[[ Placement[Note]] NL] Line[Bundle[[ Placement[Note] ]] ErrorNode[]] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a | velocity=1 +\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::ExpressionSyntax, 42, 43),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Placement[a Attrs[| AttrEntry[velocity = BinaryExpr[Literal[1] +]]] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a when x +\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::ExpressionSyntax, 36, 37),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Placement[a Gate[when BinaryExpr[NameRef[x] +]] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a: Note +\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::InlinePlacement, 34, 35),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Placement[a Body[: Block[Members[Placement[Note]] ErrorNode[+] NL]]] <]]]]",
    ),
    (
        "let a = (1 2\nClip\nlet b = )\n",
        &[
            (Code::ExpressionSyntax, 11, 12),
            (Code::PlacementSyntax, 17, 18),
            (Code::ExpressionSyntax, 26, 27),
        ],
        "File[LetDecl[let a = ParenExpr[( Literal[1]] ErrorNode[2] NL] Placement[Clip NL] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    (Note) +\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::InlinePlacement, 33, 34),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Line[ParenPlacement[( Placement[Note] )] ErrorNode[+] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    [Note] +\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::InlinePlacement, 33, 34),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Line[Bundle[[ Placement[Note] ]] ErrorNode[+] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    for i in xs: Note +\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::InlinePlacement, 44, 45),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] ForBlock[for i in NameRef[xs] : Block[Members[Placement[Note]] ErrorNode[+] NL]] <]]]]",
    ),
    (
        "Clip c:\n    [Note\n    (Note): x\n",
        &[
            (Code::BundleSyntax, 22, 23),
            (Code::InlinePlacement, 28, 29),
        ],
        "File[Placement[Clip c Body[: Block[NL > Line[Bundle[[ Placement[Note]] NL] Line[ParenPlacement[( Placement[Note] )] ErrorNode[: x] NL] <]]]]",
    ),
    (
        "let a = (1\n    let b = 2\nlet c = )\n",
        &[
            (Code::ExpressionSyntax, 15, 18),
            (Code::ExpressionSyntax, 33, 34),
        ],
        "File[LetDecl[let a = ParenExpr[( Literal[1]] ErrorNode[let b = 2] NL] LetDecl[let c = ErrorNode[)] NL]]",
    ),
    (
        "let a = (1\n    x y\nlet b = )\n",
        &[
            (Code::ExpressionSyntax, 15, 16),
            (Code::ExpressionSyntax, 27, 28),
        ],
        "File[LetDecl[let a = ParenExpr[( Literal[1]] ErrorNode[x y] NL] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "let a = (\nClip as c:\n    Note ^\n",
        &[
            (Code::ExpressionSyntax, 10, 14),
            (Code::MisplacedCaret, 30, 31),
        ],
        "File[LetDecl[let a = ParenExpr[(] NL] Placement[Clip as c Body[: Block[NL > Placement[Note ErrorNode[^] NL] <]]]]",
    ),
    (
        "let a = f(\nClip as c:\n    Note ^\n",
        &[
            (Code::ExpressionSyntax, 11, 15),
            (Code::MisplacedCaret, 31, 32),
        ],
        "File[LetDecl[let a = CallExpr[NameRef[f] ArgList[(]] NL] Placement[Clip as c Body[: Block[NL > Placement[Note ErrorNode[^] NL] <]]]]",
    ),
    (
        "let a = b[\nClip as c:\n    Note ^\n",
        &[
            (Code::ExpressionSyntax, 11, 15),
            (Code::MisplacedCaret, 31, 32),
        ],
        "File[LetDecl[let a = IndexExpr[NameRef[b] [] NL] Placement[Clip as c Body[: Block[NL > Placement[Note ErrorNode[^] NL] <]]]]",
    ),
    (
        "let a = i32(\nClip as c:\n    Note ^\n",
        &[
            (Code::ExpressionSyntax, 13, 17),
            (Code::MisplacedCaret, 33, 34),
        ],
        "File[LetDecl[let a = Conversion[i32 (] NL] Placement[Clip as c Body[: Block[NL > Placement[Note ErrorNode[^] NL] <]]]]",
    ),
    (
        "let a = [\nClip as c:\n    Note ^\n",
        &[
            (Code::ExpressionSyntax, 10, 14),
            (Code::MisplacedCaret, 30, 31),
        ],
        "File[LetDecl[let a = ArrayLit[[] NL] Placement[Clip as c Body[: Block[NL > Placement[Note ErrorNode[^] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a b +\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::InlinePlacement, 30, 31),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Line[Placement[a] Placement[b] ErrorNode[+] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a as b +\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::PlacementSyntax, 33, 34),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Placement[a as b ErrorNode[+] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a' +\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::PlacementSyntax, 29, 30),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Placement[a Flags['] ErrorNode[+] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a ' +\nlet b = )\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::ExpressionSyntax, 40, 41),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2 a ' +] NL] <]]] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "let a = (1 2\nClip' c +\n",
        &[
            (Code::ExpressionSyntax, 11, 12),
            (Code::PlacementSyntax, 21, 22),
        ],
        "File[LetDecl[let a = ParenExpr[( Literal[1]] ErrorNode[2] NL] Placement[Clip Flags['] c ErrorNode[+] NL]]",
    ),
    (
        "Clip c:\n    (Note | velocity=1 2\nClip d +\n",
        &[
            (Code::PlacementSyntax, 31, 32),
            (Code::PlacementSyntax, 40, 41),
        ],
        "File[Placement[Clip c Body[: Block[NL > Line[ParenPlacement[( Placement[Note Attrs[| AttrEntry[velocity = Literal[1]]]]] ErrorNode[2] NL] <]]] Placement[Clip d ErrorNode[+] NL]]",
    ),
    (
        "Clip c:\n    (Note | velocity=1 2\nClip as d +\n",
        &[
            (Code::PlacementSyntax, 31, 32),
            (Code::PlacementSyntax, 43, 44),
        ],
        "File[Placement[Clip c Body[: Block[NL > Line[ParenPlacement[( Placement[Note Attrs[| AttrEntry[velocity = Literal[1]]]]] ErrorNode[2] NL] <]]] Placement[Clip as d ErrorNode[+] NL]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    Plays /1: verse +\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::ExpressionSyntax, 43, 44),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] WirePlacement[Plays DefaultArg[/ Literal[1]] : Destination[BinaryExpr[NameRef[verse] +] NL]] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a/2 | x=1 +\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::ExpressionSyntax, 37, 38),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Placement[a DefaultArg[/ Literal[2]] Attrs[| AttrEntry[x = BinaryExpr[Literal[1] +]]] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a/2 when x +\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::ExpressionSyntax, 38, 39),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Placement[a DefaultArg[/ Literal[2]] Gate[when BinaryExpr[NameRef[x] +]] NL] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a/2: Note +\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::InlinePlacement, 36, 37),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2] NL] Placement[a DefaultArg[/ Literal[2]] Body[: Block[Members[Placement[Note]] ErrorNode[+] NL]]] <]]]]",
    ),
    (
        "Clip c:\n    Note/(1 2\n    a /2\nlet b = )\n",
        &[
            (Code::DefaultArgSyntax, 20, 21),
            (Code::ExpressionSyntax, 39, 40),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note DefaultArg[/ ParenExpr[( Literal[1]]] ErrorNode[2 a / 2] NL] <]]] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "Clip c:\n    Sampler/(f(\"abc\n    a | x=1 +\n",
        &[
            (Code::UnterminatedString, 23, 27),
            (Code::ExpressionSyntax, 41, 42),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Sampler DefaultArg[/ ParenExpr[( CallExpr[NameRef[f] ArgList[( StringLit[\" abc]]]]] NL] Placement[a Attrs[| AttrEntry[x = BinaryExpr[Literal[1] +]]] NL] <]]]]",
    ),
    (
        "fn f():\n",
        &[(Code::FnSyntax, 8, 8)],
        "File[FnDecl[fn f ParamList[( )] : FnBody[NL]]]",
    ),
    (
        "Clip c:\n    [Note\n    Note/(a: b)]\n",
        &[(Code::DefaultArgSyntax, 29, 30)],
        "File[Placement[Clip c Body[: Block[NL > Line[Bundle[[ Placement[Note] Placement[Note DefaultArg[/ ParenExpr[( NameRef[a]]]]] ErrorNode[: b ) ]] NL] <]]]]",
    ),
    (
        "let a = (1 2\n    (3))\nlet b = )\n",
        &[
            (Code::ExpressionSyntax, 11, 12),
            (Code::ExpressionSyntax, 30, 31),
        ],
        "File[LetDecl[let a = ParenExpr[( Literal[1]] ErrorNode[2 ( 3 ) )] NL] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "let a = 1 & (2,\nClip c:\n    Note |\nlet b = )\n",
        &[
            (Code::UnexpectedCharacter, 10, 11),
            (Code::AttributeSyntax, 34, 35),
            (Code::ExpressionSyntax, 43, 44),
        ],
        "File[LetDecl[let a = Literal[1] ErrorNode[& ( 2 ,] NL] Placement[Clip c Body[: Block[NL > Placement[Note Attrs[|] NL] <]]] LetDecl[let b = ErrorNode[)] NL]]",
    ),
    (
        "Clip c:\n    Note | velocity=(1\n  Note |\n",
        &[
            (Code::DedentMismatch, 33, 34),
            (Code::ExpressionSyntax, 33, 37),
        ],
        "File[Placement[Clip c Body[: Block[NL > Placement[Note Attrs[| AttrEntry[velocity = ParenExpr[( Literal[1]]]] NL] <]]] Placement[Note Attrs[|] NL]]",
    ),
    (
        "Clip c 2:\n    Note |\n    Note/60\n",
        &[
            (Code::PlacementSyntax, 7, 8),
            (Code::AttributeSyntax, 20, 21),
        ],
        "File[Placement[Clip c ErrorNode[2] Body[: Block[NL > Placement[Note Attrs[|] NL] Placement[Note DefaultArg[/ Literal[60]] NL] <]]]]",
    ),
    (
        "Clip c | gain=1 2:\n    Note |\n    Note/60\n",
        &[
            (Code::PlacementSyntax, 16, 17),
            (Code::AttributeSyntax, 29, 30),
        ],
        "File[Placement[Clip c Attrs[| AttrEntry[gain = Literal[1]]] ErrorNode[2] Body[: Block[NL > Placement[Note Attrs[|] NL] Placement[Note DefaultArg[/ Literal[60]] NL] <]]]]",
    ),
    (
        "Clip c when x y:\n    Note |\n    Note/60\n",
        &[
            (Code::PlacementSyntax, 14, 15),
            (Code::AttributeSyntax, 27, 28),
        ],
        "File[Placement[Clip c Gate[when NameRef[x]] ErrorNode[y] Body[: Block[NL > Placement[Note Attrs[|] NL] Placement[Note DefaultArg[/ Literal[60]] NL] <]]]]",
    ),
    (
        "Clip c | gain=1 ^:\n    Note |\n    Note/60\n",
        &[
            (Code::MisplacedCaret, 16, 17),
            (Code::AttributeSyntax, 29, 30),
        ],
        "File[Placement[Clip c Attrs[| AttrEntry[gain = Literal[1]]] ErrorNode[^] Body[: Block[NL > Placement[Note Attrs[|] NL] Placement[Note DefaultArg[/ Literal[60]] NL] <]]]]",
    ),
    (
        "Clip h:\n    for i in 0..3 4:\n        Note |\n        Note/60\n",
        &[(Code::ForSyntax, 26, 27), (Code::AttributeSyntax, 43, 44)],
        "File[Placement[Clip h Body[: Block[NL > ForBlock[for i in RangeExpr[Literal[0] .. Literal[3]] ErrorNode[4] : Block[NL > Placement[Note Attrs[|] NL] Placement[Note DefaultArg[/ Literal[60]] NL] <]] <]]]]",
    ),
    (
        "Clip h:\n    when x y:\n        Note |\n        Note/60\n",
        &[(Code::WhenSyntax, 19, 20), (Code::AttributeSyntax, 36, 37)],
        "File[Placement[Clip h Body[: Block[NL > WhenBlock[when NameRef[x] ErrorNode[y] : Block[NL > Placement[Note Attrs[|] NL] Placement[Note DefaultArg[/ Literal[60]] NL] <]] <]]]]",
    ),
    (
        "Clip h:\n    when:\n        x y:\n            Note |\n            Note/60\n",
        &[(Code::WhenSyntax, 28, 29), (Code::AttributeSyntax, 49, 50)],
        "File[Placement[Clip h Body[: Block[NL > WhenBlock[when : NL > GuardArm[NameRef[x] ErrorNode[y] : Block[NL > Placement[Note Attrs[|] NL] Placement[Note DefaultArg[/ Literal[60]] NL] <]] <] <]]]]",
    ),
    (
        "pattern P(a b):\n    Note |\n    Note/60\n",
        &[
            (Code::PatternSyntax, 12, 13),
            (Code::AttributeSyntax, 26, 27),
        ],
        "File[PatternDecl[pattern P PatternParamList[( PatternParam[a]] ErrorNode[b )] : Block[NL > Placement[Note Attrs[|] NL] Placement[Note DefaultArg[/ Literal[60]] NL] <]]]",
    ),
    (
        "fn f(a b):\n    let x = )\n    x\n",
        &[(Code::FnSyntax, 7, 8), (Code::ExpressionSyntax, 23, 24)],
        "File[FnDecl[fn f ParamList[( Param[a]] ErrorNode[b )] : FnBody[NL > LocalLet[let x = ErrorNode[)] NL] NameRef[x] NL <]]]",
    ),
    (
        "Clip c | gain=:\n    Note |\n",
        &[
            (Code::ExpressionSyntax, 14, 15),
            (Code::AttributeSyntax, 26, 27),
        ],
        "File[Placement[Clip c Attrs[| AttrEntry[gain =]] Body[: Block[NL > Placement[Note Attrs[|] NL] <]]]]",
    ),
    (
        "Clip h:\n    Plays verse:\n        Note\n",
        &[(Code::WireSyntax, 18, 23)],
        "File[Placement[Clip h Body[: Block[NL > WirePlacement[Plays ErrorNode[verse :] NL] ErrorNode[> Note NL <] <]]]]",
    ),
    (
        "Clip c 2:\nlet b = 1\n",
        &[(Code::PlacementSyntax, 7, 8)],
        "File[Placement[Clip c ErrorNode[2 :] NL] LetDecl[let b = Literal[1] NL]]",
    ),
    (
        "Clip c:\n    Plays 2: match x:\n        0: verse\n        else: |\n",
        &[(Code::WireSyntax, 18, 19), (Code::ExpressionSyntax, 61, 62)],
        "File[Placement[Clip c Body[: Block[NL > WirePlacement[Plays ErrorNode[2] : Destination[MatchBlock[match NameRef[x] : NL > MatchArm[Literal[0] : NameRef[verse] NL] ElseArm[else : ErrorNode[|] NL] <]]] <]]]]",
    ),
    (
        "let a 2 = match b:\n    0: 1\n    else: )\n",
        &[(Code::LetSyntax, 6, 7), (Code::ExpressionSyntax, 38, 39)],
        "File[LetDecl[let a ErrorNode[2] = MatchBlock[match NameRef[b] : NL > MatchArm[Literal[0] : Literal[1] NL] ElseArm[else : ErrorNode[)] NL] <]]]",
    ),
    (
        "let a = match b c:\n    0: 1\n    else: )\n",
        &[
            (Code::MatchSyntax, 16, 17),
            (Code::ExpressionSyntax, 38, 39),
        ],
        "File[LetDecl[let a = MatchBlock[match NameRef[b] ErrorNode[c :] NL > MatchArm[Literal[0] : Literal[1] NL] ElseArm[else : ErrorNode[)] NL] <]]]",
    ),
    (
        "fn f():\n    match a b:\n        0: 1\n        else: )\n",
        &[
            (Code::MatchSyntax, 20, 21),
            (Code::ExpressionSyntax, 50, 51),
        ],
        "File[FnDecl[fn f ParamList[( )] : FnBody[NL > MatchBlock[match NameRef[a] ErrorNode[b :] NL > MatchArm[Literal[0] : Literal[1] NL] ElseArm[else : ErrorNode[)] NL] <] <]]]",
    ),
];

#[test]
fn broken_sources_yield_a_tree_and_diagnostics_at_their_spans() {
    for (text, diagnostics, tree) in BROKEN {
        assert_eq!(support::codes(text), *diagnostics, "{text:?}");
        assert_eq!(support::tree(text), *tree, "{text:?}");
        assert_eq!(support::parsed(text).root().to_string(), *text);
    }
}

#[test]
fn valid_multi_line_brackets_keep_their_lines_at_any_column() {
    assert_eq!(
        support::clean_tree(
            "let x = [1,\n2,\n        3]\nlet y = f(1,\nlet_value)\nClip c:\n    Note/(1 +\n2) (Note\nwhen x)\n"
        ),
        "File[LetDecl[let x = ArrayLit[[ Literal[1] , Literal[2] , Literal[3] ]] NL] \
         LetDecl[let y = CallExpr[NameRef[f] ArgList[( Literal[1] , NameRef[let_value] )]] NL] \
         Placement[Clip c Body[: Block[NL > Line[Placement[Note DefaultArg[/ ParenExpr[( BinaryExpr[Literal[1] + Literal[2]] )]]] \
         ParenPlacement[( Placement[Note Gate[when NameRef[x]]] )] NL] <]]]]"
    );
    for (text, one_line) in [
        (
            "Clip c:\n    [Note/60\n    Note as hook /64]\n",
            "Clip c:\n    [Note/60 Note as hook /64]\n",
        ),
        (
            "Clip c:\n    [Note/60\n    Note Note]\n",
            "Clip c:\n    [Note/60 Note Note]\n",
        ),
        (
            "Clip c:\n    (Note | velocity=0.7\n    accent brightness=0.5)\n",
            "Clip c:\n    (Note | velocity=0.7 accent brightness=0.5)\n",
        ),
        (
            "Clip c:\n    [Note/60\nNote Note]\n",
            "Clip c:\n    [Note/60 Note Note]\n",
        ),
        (
            "Clip c:\n    [for i in 0..3:\n    Note/i]\n",
            "Clip c:\n    [for i in 0..3: Note/i]\n",
        ),
        (
            "Clip c:\n    [for i in\n    xs: Note/i]\n",
            "Clip c:\n    [for i in xs: Note/i]\n",
        ),
        (
            "Clip c:\n    [for i in xs\n    : Note/i]\n",
            "Clip c:\n    [for i in xs: Note/i]\n",
        ),
        (
            "Clip c:\n    [Note/60\n    Note for i in xs: Note/i]\n",
            "Clip c:\n    [Note/60 Note for i in xs: Note/i]\n",
        ),
        (
            "Clip c:\n    (\n    Note | velocity=1)\n",
            "Clip c:\n    (Note | velocity=1)\n",
        ),
        (
            "Clip c:\n    (\n    Note when x)\n",
            "Clip c:\n    (Note when x)\n",
        ),
        ("let x = [a,\nb\n]\n", "let x = [a, b]\n"),
        (
            "Clip c:\n    (\n    Note/x::y)\n",
            "Clip c:\n    (Note/x::y)\n",
        ),
        ("let x = f(1,\n(2))\n", "let x = f(1, (2))\n"),
        ("let x = [1,\n[2]]\n", "let x = [1, [2]]\n"),
        (
            "Clip c:\n    [Note/60\n    for i in xs: Note/i]\n",
            "Clip c:\n    [Note/60 for i in xs: Note/i]\n",
        ),
    ] {
        assert_eq!(support::clean_tree(text), support::clean_tree(one_line));
    }
}

#[test]
fn the_parenthesize_hint_shows_the_written_value() {
    for (text, hint) in [
        (
            "Clip c:\n    Note/f(1)\n",
            "parenthesize the whole value, as in `Note/(f(1))`",
        ),
        (
            "Clip c:\n    Note/a[0]\n",
            "parenthesize the whole value, as in `Note/(a[0])`",
        ),
        (
            "Clip c:\n    Plays/to.gain[i + 1]: d\n",
            "parenthesize the whole value, as in `Plays/(to.gain[i + 1])`",
        ),
        (
            "Clip c:\n    Note'/a[0]\n",
            "parenthesize the whole value, as in `Note'/(a[0])`",
        ),
        (
            "Clip c:\n    Note as hook /a[0]\n",
            "parenthesize the whole value, as in `Note as hook /(a[0])`",
        ),
        (
            "Clip c:\n    Note/é[0](1)[2]\n",
            "parenthesize the whole value, as in `Note/(é[0](1)[2])`",
        ),
        (
            "Clip c:\n    Note/f(1\n",
            "parenthesize the whole value, as in `Note/(…)`",
        ),
        (
            "Clip c:\n    Note/a[0\n",
            "parenthesize the whole value, as in `Note/(…)`",
        ),
        (
            "Clip c:\n    Note/a[0].b\n",
            "parenthesize the whole value, as in `Note/(a[0].b)`",
        ),
        (
            "Clip c:\n    Note/a(1).b[2]\n",
            "parenthesize the whole value, as in `Note/(a(1).b[2])`",
        ),
        (
            "Clip c:\n    Note/a[0)\n",
            "parenthesize the whole value, as in `Note/(…)`",
        ),
    ] {
        let hints: Vec<Option<String>> = support::parsed(text)
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.help.clone())
            .collect();
        assert_eq!(hints, [Some(hint.to_string())], "{text:?}");
    }
}

#[test]
fn tight_slashes_inside_a_broken_bracket_stay_in_its_region() {
    let text = format!(
        "Clip c:\n    Note/(1 2\n{}    b)\n",
        "    a/2,\n".repeat(2_500)
    );
    let codes = support::codes(&text);
    assert_eq!(codes.len(), 1, "{:?}", codes.get(..3));
    assert_eq!(codes, [(Code::DefaultArgSyntax, 20, 21)]);
}

#[test]
fn a_name_and_slash_resynchronize_only_with_a_default_value_spec_7_5_allows() {
    for (value, resynchronizes) in [
        ("x::y", true),
        ("x::y::z", false),
        ("to.gain", true),
        ("to", false),
        ("to.gain.x", true),
        ("to.gain.", false),
        ("b.c::d", false),
        ("b.c.d", true),
        ("-3", true),
        ("\"s\"", true),
        ("(1)", true),
    ] {
        let text = format!("Clip c:\n    Note/(1 2\n    a/{value}\nlet b = )\n");
        let close = text.rfind(')').expect("the last line holds `)`") as u32;
        assert_eq!(
            support::codes(&text),
            [
                (Code::DefaultArgSyntax, 20, 21),
                (Code::ExpressionSyntax, close, close + 1)
            ],
            "{text:?}"
        );
        assert_eq!(
            support::tree(&text).contains("Placement[a DefaultArg[/"),
            resynchronizes,
            "{text:?}"
        );
    }
}

#[test]
fn a_missing_closer_names_the_token_it_underlines() {
    for (text, message) in [
        (
            "let a = [1,\nlet b = 2\n",
            "expected an expression, found `let`",
        ),
        (
            "Clip c:\n    [Note\n    Plays: d]\n",
            "expected `]`, found `Plays`",
        ),
    ] {
        let messages: Vec<String> = support::parsed(text)
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.summary.clone())
            .collect();
        assert_eq!(
            messages.first().map(String::as_str),
            Some(message),
            "{text:?}"
        );
    }
}

#[test]
fn recovery_resynchronizes_at_state_lines_and_treats_a_history_read_as_an_expression_bracket() {
    let rows: &[(&str, Code, &str, &str)] = &[
        (
            "input x = 1\nlet a = (1 +\nstate s = x{1}\nClip c:\n    Note/s\n",
            Code::ExpressionSyntax,
            "state",
            "StateDecl[state s",
        ),
        (
            "input x = 1\nstate s = x{1, 0\nlet y = 2\nClip c:\n    Note/y\n",
            Code::ExpressionSyntax,
            "let",
            "LetDecl[let y",
        ),
        (
            "input x = 1\nstate s = x{1, 0\nClip c:\n    Note/s\n",
            Code::ExpressionSyntax,
            "Clip",
            "Placement[Clip c",
        ),
        (
            "Clip c:\n    Note | velocity=(1\n  state s = 1\nClip d:\n    Note/s\n",
            Code::DedentMismatch,
            "s",
            "StateDecl[state s",
        ),
    ];
    for (text, code, at, item) in rows {
        let (start, end) = support::at(text, at, 0);
        assert_eq!(support::codes(text), vec![(*code, start, end)], "{text}");
        assert!(support::tree(text).contains(item), "{text}");
        let codes: Vec<Code> = support::checked(text)
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect();
        assert_eq!(codes, vec![*code], "{text}");
    }
}

#[test]
fn the_text_after_a_faulty_bracket_lexes_as_it_would_after_a_correct_one() {
    let clips: String = (1..=3)
        .map(|n| format!("Clip c{n}:\n    Note/62 | velocity=0.7\n              brightness=0.5\n"))
        .collect();
    let rows: &[(&str, &str, &str, &str)] = &[
        ("input x = 1\nstate s = ", "x{1]", "x{1}", "]"),
        ("input x = 1\nstate s = ", "x{1)", "x{1}", ")"),
        ("input x = 1\nstate s = ", "x{(1}", "x{(1)}", "}"),
        ("input x = 1\nstate s = ", "(x{1}]", "(x{1})", "]"),
        ("input x = 1\nstate s = ", "[x{1})", "[x{1}]", ")"),
        ("input x = 1\nlet z = ", "x{1]", "x{1}", "]"),
        ("input x = 1\nlet z = ", "x{(1}", "x{(1)}", "}"),
        ("input x = 1\nlet z = ", "(1}", "(1)", "}"),
    ];
    for (before, faulty, correct, closer) in rows {
        for (head, tail) in [
            (format!("{before}{faulty}\nlet y = 2\n"), clips.clone()),
            (
                format!("let y = 2\n{}{before}{faulty}\n", &clips[..clips.len() / 3]),
                clips[clips.len() / 3..].to_string(),
            ),
        ] {
            let text = format!("{head}{tail}");
            let fixed = format!("{}{tail}", head.replacen(faulty, correct, 1));
            let after = |text: &str, from: usize| -> Vec<(SyntaxKind, String)> {
                lex(&support::source(text), &vocabulary())
                    .tokens
                    .iter()
                    .filter(|token| token.range.start as usize >= from)
                    .map(|token| {
                        let range = token.range.start as usize..token.range.end as usize;
                        (token.kind, text[range].to_string())
                    })
                    .collect()
            };
            assert_eq!(
                after(&text, head.len()),
                after(&fixed, fixed.len() - tail.len()),
                "{text}"
            );
            let start = head.find(faulty).unwrap_or(0) + faulty.rfind(closer).unwrap_or(0);
            let parsed: Vec<(Code, u32, u32)> = support::codes(&text);
            assert_eq!(parsed.len(), 1, "{text}{parsed:?}");
            assert_eq!(parsed[0].1 as usize, start, "{text}{parsed:?}");
        }
    }
    for faulty in ["x{1]", "x{1)", "x{(1}"] {
        let text = format!("input x = 1\nstate s = {faulty}\nlet y = 2\n{clips}");
        let found: Vec<Code> = support::checked(&text)
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect();
        assert_eq!(found, vec![Code::ExpressionSyntax], "{text}");
    }
}

#[test]
fn a_name_inside_a_faulty_part_of_a_kept_item_is_a_placeholder_wherever_it_is_used() {
    let rows: &[support::Row] = &[
        (
            &[
                "Clip c:\n    Note/60 as n\n    Note | velocity=c.n.velocity\n",
                "Clip d | gain=c.n.velocity:\n    Note\n",
            ],
            &[(Code::PlacementSyntax, "as")],
        ),
        (
            &["Synth s:\n    Osc/220 as o\n    Filter:\n        Feeds: o\n"],
            &[(Code::PlacementSyntax, "as")],
        ),
        (
            &["Clip c:\n    Plays/0.5 as w: c\n    Note | velocity=c.w.gain\n"],
            &[(Code::WireSyntax, "as")],
        ),
        (
            &["Clip c:\n    [Note/60 as n, Note]\n    Note | velocity=c.n.velocity\n"],
            &[(Code::BundleSyntax, "as")],
        ),
        (
            &["Clip c as k:\n    Note\n", "Clip d:\n    Plays: k\n"],
            &[(Code::PlacementSyntax, "as")],
        ),
        (
            &["Clip c:\n    Note as n /60\n    Note | velocity=c.n.velocity\n"],
            &[],
        ),
        (
            &["Clip c:\n    Note\n    Note | velocity=c.m.velocity\n"],
            &[(Code::UnknownAttribute, "m")],
        ),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
}

#[test]
fn a_state_header_cut_by_a_fault_declares_its_name_as_a_placeholder() {
    let input = "input a = 0\n";
    let read = "Clip c:\n    Note/(60 + t)\n";
    let rows: &[support::Row] = &[
        (
            &[input, "state[1 t = t{1, 0} + a\n", read],
            &[(Code::StateSyntax, "t")],
        ),
        (
            &[input, "state[ t = t{1, 0} + a\n", read],
            &[(Code::StateSyntax, "t")],
        ),
        (
            &[input, "state[x] t = t{1, 0} + a\n", read],
            &[(Code::StateSyntax, "x")],
        ),
        (
            &[
                input,
                "state[1 t = t{1, 0} + a\n",
                "Clip c:\n    Note when t > 0\n",
            ],
            &[(Code::StateSyntax, "t")],
        ),
        (
            &[
                input,
                "state[1 t = t{1, 0} + a\n",
                "state u = t + 1\n",
                "Clip c:\n    Note/(60 + u)\n",
            ],
            &[(Code::StateSyntax, "t")],
        ),
        (&[input, "state[1] t = t{1, 0} + a\n", read], &[]),
        (&[input, "state t = a{1, 0}\n", read], &[]),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
}

#[test]
fn every_top_level_header_cut_by_a_fault_declares_its_names_as_placeholders() {
    let read_a = "Clip c:\n    Note/(60 + a)\n";
    let read_x = "Clip c:\n    Note/(60 + x)\n";
    let call_f = "Clip c:\n    Note/(60 + f(1))\n";
    let place_p = "Clip c:\n    P\n";
    let rows: &[support::Row] = &[
        (&["input [a = 0\n", read_a], &[(Code::InputSyntax, "[")]),
        (&["let [x = 5\n", read_x], &[(Code::LetSyntax, "[")]),
        (&["let (x = 5\n", read_x], &[(Code::LetSyntax, "(")]),
        (&["fn (f(a): a\n", call_f], &[(Code::FnSyntax, "(")]),
        (
            &["pattern (P():\n    Note\n", place_p],
            &[(Code::PatternSyntax, "(")],
        ),
        (
            &["fn f(n) [3 else 0: f(n - 1)\n", call_f],
            &[(Code::FnSyntax, "[")],
        ),
        (&["input a: i32[ = 0\n", read_a], &[(Code::TypeSyntax, "=")]),
        (&["let x: i32[ = 5\n", read_x], &[(Code::TypeSyntax, "=")]),
        (&["input a 0\n", read_a], &[(Code::InputSyntax, "0")]),
        (&["input a =\n", read_a], &[(Code::ExpressionSyntax, "\n")]),
        (&["let x 5\n", read_x], &[(Code::LetSyntax, "5")]),
        (&["fn f(a b): a\n", call_f], &[(Code::FnSyntax, "b")]),
        (
            &["pattern P(a b):\n    Note/a\n", "Clip c:\n    P | a=1\n"],
            &[(Code::PatternSyntax, "b")],
        ),
        (&["fn f(a) a\n", call_f], &[(Code::FnSyntax, "a")]),
        (
            &["pattern P()\n    Note\n", place_p],
            &[(Code::PatternSyntax, "\n")],
        ),
        (&["fn f(a = (1): a\n", call_f], &[(Code::FnSyntax, ":")]),
        (
            &[
                "pattern P(default r = (1):\n    Note/r\n",
                "Clip c:\n    P/60\n",
            ],
            &[(Code::PatternSyntax, ":")],
        ),
        (
            &["Clip [c:\n    Note\n", "Clip d | gain=c.gain:\n    Note\n"],
            &[(Code::PlacementSyntax, "[")],
        ),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
}

#[test]
fn a_missing_closer_marks_the_expression_it_closes_and_not_the_next_item() {
    let compiler = symphonyscript::Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    type Row<'a> = (&'a [&'a str], &'a [Code], &'a [(Code, &'a str)]);
    let rows: [Row; 4] = [
        (
            &["Clip c:\n    Note/(1 + 2\n", "Clip d:\n    Note | loud=1\n"],
            &[Code::DefaultArgSyntax, Code::UnknownAttribute],
            &[(Code::UnknownAttribute, "loud")],
        ),
        (
            &["let a = (1 + 2\n", "Clip z:\n    Note | loud=1\n"],
            &[Code::ExpressionSyntax, Code::UnknownAttribute],
            &[(Code::UnknownAttribute, "loud")],
        ),
        (
            &[
                "Clip c:\n    Note/(1 +\n    Note/2\n",
                "Clip d:\n    Note\n",
            ],
            &[Code::DefaultArgSyntax],
            &[],
        ),
        (
            &[
                "Clip c:\n    Note/(1 +\n        2)\n",
                "Clip d:\n    Note\n",
            ],
            &[],
            &[],
        ),
    ];
    for (items, codes, spanned) in rows {
        for order in support::permutations(items) {
            let text = order.concat();
            let found = support::error_texts(&compiler, &text);
            let mut found_codes: Vec<Code> = found.iter().map(|(code, _)| *code).collect();
            found_codes.sort();
            let mut wanted = codes.to_vec();
            wanted.sort();
            assert_eq!(found_codes, wanted, "{text}");
            for (code, piece) in spanned {
                assert!(
                    found.contains(&(*code, (*piece).to_string())),
                    "{text}{found:?}"
                );
            }
        }
    }
}
