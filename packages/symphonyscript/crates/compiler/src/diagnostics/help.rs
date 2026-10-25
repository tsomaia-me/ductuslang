use super::code::Code;

pub(super) fn help(code: Code) -> Option<&'static str> {
    Some(match code {
        Code::TabIndentation => "indent with spaces",
        Code::DedentMismatch => "indent the line to the column of an enclosing line",
        Code::Semicolon => "end the line without `;`",
        Code::ReservedWord => "choose another name",
        Code::ReservedToken => "compare with `is` or `is not`",
        Code::MisplacedCaret => {
            "write `^` only at the start of an index or a slice bound, as in `xs[^1]`"
        }
        Code::MisplacedUnderscore => "write `_` only between two digits, as in `1_000`",
        Code::FloatDigits => "write digits on both sides of the point, as in `0.5`",
        Code::FloatTypeSuffix => "give a float literal the suffix `f32` or `f64`",
        Code::UnterminatedString => "close the string with `\"` on its line",
        Code::UnexpectedIndent => "indent a line only under a line that opens a block",
        Code::ChainedComparison => "join the comparisons with `and`, as in `a < b and b < c`",
        Code::UnparenthesizedIf => "parenthesize the `if`",
        Code::MatchPosition => "make the `match` the whole value of a `let`",
        Code::MixedArguments => "pass every argument by position or every one by name",
        Code::ConversionSyntax => "parenthesize the converted value, as in `i32(x)`",
        Code::MisplacedHistory => "read the history in a state's initializer",
        Code::HistoryTarget => "read the history of an input, a state or a reactive `let`",
        Code::HistoryFallback => "state the fallback, as in `x{1, 0}`",
        Code::HistoryOffset => {
            "write the offset as a positive integer known at compile time, as in `x{1, 0}`"
        }
        Code::UnboundedState => {
            "slice the initializer, as in `(s{1, []} + [x])[^8..]`, to keep its newest elements"
        }
        Code::TopLevelWire => "place the wire inside its source's body",
        Code::DefaultArgSyntax => "parenthesize the value, as in `Note/(base + 7)`",
        Code::InlinePlacement => "parenthesize the placement or write it on its own line",
        Code::DuplicateName => "rename one of the two declarations",
        Code::CompileTimeNative => "pass arguments known at compile time",
        Code::ChildNameClash => "rename one of the two",
        Code::ForVariableClash => "rename the `for` variable",
        Code::DuplicateLocal => "rename one of the two",
        Code::ParameterOrder => "move the parameters that have defaults after the others",
        Code::NeedlessBound => "remove the bound",
        Code::DuplicateDefaultParameter => "mark one parameter `default`",
        Code::NamedInFor => "remove the `as` name",
        Code::ZeroLength => "write a length or capacity of at least 1",
        Code::NamedBuiltinArguments => "pass the arguments by position",
        Code::DuplicateArgument => "supply the parameter once",
        Code::DuplicateAttribute => "set the attribute once",
        Code::MissingAttribute => "set the attribute",
        Code::DuplicateArm => "remove the repeated arm",
        Code::MatchIncomplete => "add the missing arms or an `else:` arm",
        Code::RedundantElse => "remove the `else:` arm",
        Code::FloatToInteger => "convert explicitly, as in `i32(x)`",
        Code::ReactiveRecursion => "declare a bound on the function, as in `bounded 10 else 0`",
        Code::UnexpectedCharacter
        | Code::ClauseColumn
        | Code::InvalidIdentifier
        | Code::UnknownSuffix
        | Code::PrefixedSuffix
        | Code::IntegerTooLarge
        | Code::InvalidEscape
        | Code::TopLevelSyntax
        | Code::TypeSyntax
        | Code::ExpressionSyntax
        | Code::MatchSyntax
        | Code::InputSyntax
        | Code::LetSyntax
        | Code::FnSyntax
        | Code::PatternSyntax
        | Code::StateSyntax
        | Code::HistoryDepth
        | Code::StateValue
        | Code::PlacementSyntax
        | Code::FlagSyntax
        | Code::AttributeSyntax
        | Code::BundleSyntax
        | Code::ForSyntax
        | Code::WireSyntax
        | Code::MisplacedWire
        | Code::WhenSyntax
        | Code::VariantName
        | Code::PreludeItem
        | Code::RecursivePattern
        | Code::UnknownName
        | Code::UnknownKind
        | Code::MisplacedTo
        | Code::UnknownType
        | Code::LengthLimit
        | Code::VariableArray
        | Code::CapacityExceeded
        | Code::InterpolationType
        | Code::UnknownVariant
        | Code::ArrayOperand
        | Code::ConditionType
        | Code::TypeMismatch
        | Code::RaggedArray
        | Code::MatchScrutinee
        | Code::DeclarationCycle
        | Code::NotAValue
        | Code::NotCallable
        | Code::ArgumentCount
        | Code::UnknownParameter
        | Code::MissingArgument
        | Code::ReactiveInitializer
        | Code::InputValue
        | Code::UnknownAttribute
        | Code::RuntimePath
        | Code::IndexType
        | Code::NotAnArray
        | Code::IndexOutOfRange
        | Code::DestinationType
        | Code::ExpansionLimit
        | Code::InlineLimit
        | Code::UseForm
        | Code::NoDefaultParameter
        | Code::UnknownFlag
        | Code::NoDefaultAttribute
        | Code::TopLevelPattern
        | Code::EmptyBundle
        | Code::BundleMember
        | Code::IterableType
        | Code::ReactiveIterable
        | Code::RangeBound
        | Code::UnitOperation
        | Code::OperandType
        | Code::ConversionType
        | Code::ReactiveString
        | Code::MixedFamilies
        | Code::MatchLabel
        | Code::ToCandidates
        | Code::RecursionDepth
        | Code::RecursionLimit
        | Code::ReactiveNodeReference
        | Code::DoesNotFit
        | Code::IntegerOverflow
        | Code::ConstantOverflow
        | Code::UnitMismatch
        | Code::PlainInFamily
        | Code::ConflictingUnits
        | Code::InconsistentUses
        | Code::InconsistentShapes
        | Code::IntegerInput
        | Code::NotTopLevel
        | Code::CountExceeded
        | Code::CountBelowMinimum
        | Code::ChildNotAccepted
        | Code::WireEndpoint
        | Code::WireCycle
        | Code::UnreachableInstance
        | Code::NoEntryInstance
        | Code::InternalError => return None,
    })
}
