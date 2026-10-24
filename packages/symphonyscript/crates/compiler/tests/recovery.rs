mod support;

use symphonyscript::Code;

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
        "let a = [ ]\n",
        &[(Code::ExpressionSyntax, 10, 11)],
        "File[LetDecl[let a = ArrayLit[[ ]] NL]]",
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
];

#[test]
fn broken_sources_yield_a_tree_and_diagnostics_at_their_spans() {
    for (text, diagnostics, tree) in BROKEN {
        assert_eq!(support::codes(text), *diagnostics, "{text:?}");
        assert_eq!(support::tree(text), *tree, "{text:?}");
        assert_eq!(support::parsed(text).syntax().to_string(), *text);
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
}
