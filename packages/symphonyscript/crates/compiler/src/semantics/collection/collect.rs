use std::collections::BTreeMap;

use symphonyscript_core::Vocabulary;

use super::builtin_name::BuiltinName;
use super::collected::Collected;
use super::decl::Decl;
use super::inline_counts::InlineCounts;
use super::namespace::Namespace;
use super::pattern_graph::PatternGraph;
use super::recursive_functions::recursive_functions;
use super::variants::Variants;
use crate::diagnostics::Code;
use crate::semantics::ir::{Hir, Ident, Loc, Top};
use crate::semantics::reporting::Report;

pub(crate) fn collect(
    hir: &Hir,
    vocabulary: &Vocabulary,
    variants: &Variants,
    report: &mut Report,
) -> Collected {
    let base = base_namespace(vocabulary);
    let mut prelude = base.clone();
    let mut source = base.clone();
    let mut first: BTreeMap<String, Loc> = BTreeMap::new();
    let mut prelude_first: BTreeMap<String, Loc> = BTreeMap::new();
    for (position, top) in hir.top.iter().enumerate() {
        let in_prelude = hir.is_prelude_top(position);
        let (name, decl) = match *top {
            Top::Input(id) => {
                if in_prelude {
                    let loc = hir.input(id).loc;
                    report.error(
                        hir,
                        Code::PreludeItem,
                        loc,
                        "the prelude holds `let`, `fn` and `pattern` declarations only",
                    );
                    continue;
                }
                (hir.input(id).name.clone(), Decl::Input(id))
            }
            Top::Let(id) => (hir.let_decl(id).name.clone(), Decl::Let(id)),
            Top::Fn(id) => (hir.fn_decl(id).name.clone(), Decl::Fn(id)),
            Top::Pattern(id) => (hir.pattern(id).name.clone(), Decl::Pattern(id)),
            Top::Placement(id) => {
                if in_prelude {
                    let loc = hir.placement(id).loc;
                    report.error(
                        hir,
                        Code::PreludeItem,
                        loc,
                        "the prelude holds `let`, `fn` and `pattern` declarations only",
                    );
                    continue;
                }
                (hir.placement(id).name.clone(), Decl::Instance(id))
            }
        };
        let Some(name) = name else {
            continue;
        };
        variant_name(hir, vocabulary, variants, &name, report);
        if in_prelude {
            if declare(
                hir,
                &base,
                &mut prelude,
                &mut prelude_first,
                &name,
                decl,
                report,
            ) {
                source.insert(name.text.clone(), decl);
            }
        } else {
            declare(hir, &base, &mut source, &mut first, &name, decl, report);
        }
    }
    let recursive = recursive_functions(hir, &source, &prelude);
    let inline = InlineCounts::of(hir, &source, &prelude, &recursive);
    let graph = PatternGraph::of(hir, &source, &prelude, report);
    Collected {
        source,
        prelude,
        recursive,
        inline,
        cyclic_patterns: graph.cyclic,
        stand_ins: graph.stand_ins,
    }
}

fn base_namespace(vocabulary: &Vocabulary) -> Namespace {
    let mut base = Namespace::default();
    for index in 0..vocabulary.node_kinds().len() {
        if let (Some(kind), Some(decl)) = (
            vocabulary.node_kind_at(index),
            vocabulary.node_kinds().get(index),
        ) {
            base.insert(decl.name.clone(), Decl::Node(kind));
        }
    }
    for index in 0..vocabulary.wire_kinds().len() {
        if let (Some(kind), Some(decl)) = (
            vocabulary.wire_kind_at(index),
            vocabulary.wire_kinds().get(index),
        ) {
            base.insert(decl.name.clone(), Decl::Wire(kind));
        }
    }
    for index in 0..vocabulary.enums().len() {
        if let (Some(id), Some(decl)) = (vocabulary.enum_at(index), vocabulary.enums().get(index)) {
            base.insert(decl.name.clone(), Decl::Enum(id));
        }
    }
    for builtin in BuiltinName::all() {
        base.insert(builtin.name().to_string(), Decl::Builtin(builtin));
    }
    base
}

fn declare(
    hir: &Hir,
    base: &Namespace,
    namespace: &mut Namespace,
    first: &mut BTreeMap<String, Loc>,
    name: &Ident,
    decl: Decl,
    report: &mut Report,
) -> bool {
    if let Some(existing) = base.get(&name.text) {
        let what = match existing {
            Decl::Builtin(_) => "a built-in name",
            Decl::Node(_) => "a node kind of the vocabulary",
            Decl::Wire(_) => "a wire kind of the vocabulary",
            Decl::Enum(_) => "an enum of the vocabulary",
            Decl::Input(_) | Decl::Let(_) | Decl::Fn(_) | Decl::Pattern(_) | Decl::Instance(_) => {
                "already declared"
            }
        };
        report.error(
            hir,
            Code::DuplicateName,
            name.loc,
            format!("`{}` is {what}", name.text),
        );
        return false;
    }
    if let Some(earlier) = first.get(&name.text) {
        report.error_with(
            hir,
            Code::DuplicateName,
            name.loc,
            &[*earlier],
            format!("`{}` is declared twice", name.text),
        );
        return false;
    }
    first.insert(name.text.clone(), name.loc);
    namespace.insert(name.text.clone(), decl);
    true
}

fn variant_name(
    hir: &Hir,
    vocabulary: &Vocabulary,
    variants: &Variants,
    name: &Ident,
    report: &mut Report,
) {
    if let Some(owner) = variants
        .owner(&name.text)
        .and_then(|id| vocabulary.enum_decl(id))
        .map(|decl| decl.name.as_str())
    {
        report.error(
            hir,
            Code::VariantName,
            name.loc,
            format!(
                "`{}` is a variant of the enum `{owner}` and is never a declared name",
                name.text
            ),
        );
    }
}
