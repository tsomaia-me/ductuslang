use super::code::Code;

pub(crate) fn stray_closer_help(closer: char) -> Option<String> {
    let opener = opener_of(closer)?;
    Some(format!(
        "remove the `{closer}`, or match it with an opening `{opener}`"
    ))
}

pub(crate) fn mismatched_closer_help(found: char, expected: char) -> Option<String> {
    let opener = opener_of(expected)?;
    opener_of(found)?;
    Some(format!(
        "remove the `{found}`, or replace it with the `{expected}` that closes the open `{opener}`"
    ))
}

pub(crate) fn index_outside_help(length: u64) -> Option<String> {
    (length == 0).then(|| {
        String::from(
            "the array is empty: give it elements, or read the element only where `len(…)` is above 0",
        )
    })
}

pub(crate) fn for_start_and_body_help() -> String {
    String::from(
        "give the start and the body values of one type and shape, as the two arms of an `if` are",
    )
}

pub(crate) fn accumulator_capacity_help(annotated: i128, holds: i128) -> String {
    let kept = if annotated == 1 {
        String::from("its newest element with `[^1..]`")
    } else {
        format!("its newest {annotated} elements with `[^{annotated}..]`")
    };
    format!("slice the body's value to {kept}, or annotate a capacity of at least {holds}")
}

fn opener_of(closer: char) -> Option<char> {
    Some(match closer {
        ')' => '(',
        ']' => '[',
        '}' => '{',
        _ => return None,
    })
}

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
        Code::UnparenthesizedFor => {
            "parenthesize the `for` expression, as in `1 + (for x in held into sum from 0: sum + x)`"
        }
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
        Code::ForVariableClash => "rename the name the `for` binds",
        Code::DuplicateLocal => "rename one of the two",
        Code::ParameterOrder => "move the parameters that have defaults after the others",
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
        Code::ReactiveRecursion => {
            "write the recursion over the cell as a `for` expression, as in `for x in held into low from 127: min(low, x)`"
        }
        Code::UnexpectedCharacter => {
            "remove the character, or write the word the language uses, as in `and`"
        }
        Code::ClauseColumn => {
            "start the continued line at the column of the clause's first entry, counting each character as one"
        }
        Code::InvalidIdentifier => "give the name a letter or a digit, as in `unused`",
        Code::UnknownSuffix => "write a type suffix or a unit of the vocabulary, as in `2bars`",
        Code::PrefixedSuffix => {
            "write only digits of the prefix's base and no suffix, as in `0b10`"
        }
        Code::IntegerTooLarge => "write an integer whose magnitude is below 2¹²⁷",
        Code::InvalidEscape => "write one of the escapes `\\\"`, `\\\\`, `\\n`, `\\t` and `\\{`",
        Code::TopLevelSyntax => "declare the value, as in `let five = 5`",
        Code::PreludePath => "give the source file a path that no prelude file holds",
        Code::TypeSyntax => {
            "write the lengths and the capacity in digits, as in `i32[3]` or `i32[..10]`"
        }
        Code::ExpressionSyntax => "give every operator its operands and close every bracket",
        Code::MatchSyntax => "end the `match` line with `:` and indent its arms under it",
        Code::InputSyntax => {
            "write `input`, a name, `=` and the initial value, as in `input level = 1`"
        }
        Code::LetSyntax => "write `let`, a name, `=` and the value, as in `let one = 1`",
        Code::FnSyntax => {
            "write `fn`, a name, its parameters, `:` and the body, as in `fn double(x): x * 2`"
        }
        Code::PatternSyntax => {
            "write `pattern`, a name, its parameters, `:` and an indented body, as in `pattern Hit(x):`"
        }
        Code::StateSyntax => {
            "declare the state at top level as `state`, a name, `=` and its initializer, as in `state count = 1`"
        }
        Code::HistoryDepth => "write `[N]` with N the deepest offset the state reads",
        Code::StateValue => {
            "hold a number, a `bool`, an enum value or an array of them in the state"
        }
        Code::PlacementSyntax => {
            "write the kind, then `as` and a name, a `/` value, a `when` gate and `|` attributes, in that order, as in `Note as n /60 | velocity=0.9`"
        }
        Code::FlagSyntax => "end the flags before the next name, as in `Note'`",
        Code::AttributeSyntax => "write each entry as `name=value`, `name` or `!name`",
        Code::BundleSyntax => "close the bundle with `]`, as in `[Note Note]`",
        Code::ForSyntax => "write `for <name> in <iterable>:` with one name or two",
        Code::WireSyntax => "end the wire with `:` and its destination, as in `Plays: c`",
        Code::MisplacedWire => "place the wire on its own line, outside the bundle",
        Code::WhenSyntax => "end the condition with `:` and indent the block under it",
        Code::VariantName => "choose a name that is no variant of a vocabulary enum",
        Code::PreludeItem => "declare only `let`, `fn` and `pattern` in the prelude",
        Code::RecursivePattern => "end the chain of uses so that no pattern uses itself",
        Code::UnknownName => "declare the name, or correct its spelling",
        Code::UnknownKind => "use a kind of the vocabulary, or correct its spelling",
        Code::MisplacedTo => {
            "read `to` with an attribute path inside a wire placement, as in `Plays | gain=to.gain: a`"
        }
        Code::UnknownType => "use a scalar type or an enum of the vocabulary",
        Code::LengthLimit => "keep the length or capacity below 2³¹, as in `i32[..2147483647]`",
        Code::VariableArray => {
            "annotate the value as a variable array, as in `i32[..4]`, or bind it to an attribute that takes one"
        }
        Code::CapacityExceeded => "slice the value to the capacity, as in `held[^8..]`",
        Code::InterpolationType => {
            "interpolate a number, a `bool`, an enum variant, a string or a unit value"
        }
        Code::UnknownVariant => "use a variant the enum declares, as in `Wave::Saw`",
        Code::ArrayOperand => {
            "join two arrays with `+`, as in `[1, 2] + [3]`, or apply the operator to their elements"
        }
        Code::ConditionType => "compare to make a `bool`, as in `x > 0`",
        Code::TypeMismatch => "give the position a value of the type it expects",
        Code::RaggedArray => {
            "make every element a single value or an array of one length, as in `[[1], [2]]`"
        }
        Code::MatchScrutinee => "match on an enum, an integer or a `bool`",
        Code::DeclarationCycle => "break the cycle so that no declaration reads itself",
        Code::NotAValue => "use a value here, such as a number, a `let` or a call",
        Code::NotCallable => "call a function, as in `abs(2)`",
        Code::ArgumentCount => "pass one argument for each parameter that has no default",
        Code::UnknownParameter => "set a parameter the pattern declares",
        Code::MissingArgument => "supply the parameter, as in `P | root=60`",
        Code::ReactiveInitializer => {
            "initialize the input with a compile-time value, such as a `let`"
        }
        Code::InputValue => "hold the string in a `let`",
        Code::UnknownAttribute => "use an attribute the kind declares, or correct its spelling",
        Code::RuntimePath => {
            "start the path at an instance known at compile time, as in `s[1].gain`"
        }
        Code::IndexType => "index with an integer",
        Code::NotAnArray => "index, slice or measure an array, not a single value",
        Code::IndexOutOfRange => "keep the index inside the array, from 0 to its length minus one",
        Code::DestinationType => "name an instance as the destination",
        Code::ExpansionLimit => "lower the loop's bound or the number of uses",
        Code::InlineLimit => {
            "compute a repeated value once, as in `h0(x) * 10`, or lower the bound"
        }
        Code::UseForm => "move the flags and the body into the pattern",
        Code::NoDefaultParameter => "mark a parameter `default`, or set the parameter by name",
        Code::UnknownFlag => "use a flag the kind assigns, as in `Note'`",
        Code::NoDefaultAttribute => "set the attribute by name in the clause",
        Code::TopLevelPattern => "wrap the pattern's placements in one placement",
        Code::EmptyBundle => "give the bundle at least one member",
        Code::BundleMember => "make the bundle's members placements without bodies",
        Code::IterableType => "iterate over a range or an array, as in `0..5`",
        Code::ReactiveIterable => {
            "iterate over compile-time values and read the cell inside the body"
        }
        Code::RangeBound => "bound the range with integers, as in `0..4`",
        Code::UnitOperation => {
            "multiply or divide a unit value by a plain number, as in `2bars * 2`"
        }
        Code::OperandType => {
            "give the operator operands of the types it takes, or convert them, as in `i32(x)`"
        }
        Code::ConversionType => "convert a number, or choose one with `if`",
        Code::ReactiveString => "build the string from compile-time values, such as a `let`",
        Code::MixedFamilies => "combine values of one family, as in `2bars + 1bars`",
        Code::MatchLabel => {
            "label the arm with a variant of the scrutinee's enum or a literal of its type"
        }
        Code::ToCandidates => "give every candidate the attribute with one type and unit family",
        Code::RecursionDepth => "let the recursion reach its base case within the limit",
        Code::RecursionLimit => "reduce the distinct arguments the recursion computes",
        Code::LimitWarning => {
            "reduce the work the limit counts, or raise its warning value in the package's `options`"
        }
        Code::ReactiveNodeReference => {
            "choose the destination in the wire itself, as in `Plays: [verse, chorus][i]`"
        }
        Code::DoesNotFit => "use a value that fits the type, or convert it explicitly",
        Code::IntegerOverflow => "keep the integer's magnitude below 2¹²⁷",
        Code::ConstantOverflow => {
            "keep the operation inside its type's range, or widen its operands"
        }
        Code::UnitMismatch => "use a unit of the attribute's family, as in `2bars`",
        Code::PlainInFamily => "give the plain number a unit of the family, as in `1ms`",
        Code::ConflictingUnits => "give the plain number a unit, as in `1bars`",
        Code::InconsistentUses => {
            "use a value equal as an integer and as a float, or split it into two `let`s"
        }
        Code::InconsistentShapes => "make the call yield one shape in every type",
        Code::IntegerInput => "write the initial value as a float, as in `4.0`",
        Code::NotTopLevel => "place the instance inside a parent that accepts it",
        Code::CountExceeded => "remove the instances past the count",
        Code::CountBelowMinimum => "add the instances the count requires",
        Code::ChildNotAccepted => "place the child in a parent that accepts its kind",
        Code::WireEndpoint => "connect kinds the wire kind connects",
        Code::WireCycle => "break the cycle, or use a wire kind marked cyclic",
        Code::UnreachableInstance => {
            "reach the instance from an entry-kind instance, through a child or a wire"
        }
        Code::NoEntryInstance => "place an instance of an entry kind, as in `Session s`",
        Code::InternalError => "report the compiler defect with the program that drew it",
        Code::UnbuiltConstruct => "write the program without the construct the summary names",
    })
}
