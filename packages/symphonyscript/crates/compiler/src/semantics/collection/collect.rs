use std::collections::BTreeMap;

use symphonyscript_core::Vocabulary;

use super::builtin_name::BuiltinName;
use super::cell_groups::CellGroups;
use super::collected::Collected;
use super::decl::Decl;
use super::inline_counts::InlineCounts;
use super::namespace::Namespace;
use super::pattern_graph::PatternGraph;
use super::recursive_functions::recursive_functions;
use super::variants::Variants;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::{Hir, Ident, Loc, Top};
use crate::semantics::reporting::Report;

pub(crate) fn collect(
    hir: &Hir,
    vocabulary: &Vocabulary,
    variants: &Variants,
    report: &mut Report,
) -> Result<Collected, Internal> {
    for (code, loc, summary) in &hir.drawn {
        report.error(hir, *code, *loc, summary.clone());
    }
    let base = base_namespace(vocabulary)?;
    let mut prelude = base.clone();
    let mut source = base.clone();
    for (index, native) in vocabulary.natives().iter().enumerate() {
        let id = vocabulary
            .native_at(index)
            .ok_or(Internal::missed("native function"))?;
        prelude.insert(native.name.clone(), Decl::Native(id));
        source.insert(native.name.clone(), Decl::Native(id));
    }
    let mut first: BTreeMap<String, Loc> = BTreeMap::new();
    let mut prelude_first: BTreeMap<String, Loc> = BTreeMap::new();
    for (position, top) in hir.top.iter().enumerate() {
        let in_prelude = hir.is_prelude_top(position);
        let (name, decl) = match *top {
            Top::Input(id) => {
                if in_prelude {
                    let loc = hir.input(id)?.loc;
                    report.error(
                        hir,
                        Code::PreludeItem,
                        loc,
                        "the prelude holds `let`, `fn` and `pattern` declarations only",
                    );
                    continue;
                }
                (hir.input(id)?.name.clone(), Some(Decl::Input(id)))
            }
            Top::Let(id) => (hir.let_decl(id)?.name.clone(), Some(Decl::Let(id))),
            Top::State(id) => {
                if in_prelude {
                    let loc = hir.state(id)?.loc;
                    report.error(
                        hir,
                        Code::PreludeItem,
                        loc,
                        "the prelude holds `let`, `fn` and `pattern` declarations only",
                    );
                    continue;
                }
                (hir.state(id)?.name.clone(), Some(Decl::State(id)))
            }
            Top::Fn(id) => (hir.fn_decl(id)?.name.clone(), Some(Decl::Fn(id))),
            Top::Pattern(id) => (hir.pattern(id)?.name.clone(), Some(Decl::Pattern(id))),
            Top::Placeholder(index) => (hir.placeholders.get(index as usize).cloned(), None),
            Top::Placement(id) => {
                if in_prelude {
                    let loc = hir.placement(id)?.loc;
                    report.error(
                        hir,
                        Code::PreludeItem,
                        loc,
                        "the prelude holds `let`, `fn` and `pattern` declarations only",
                    );
                    continue;
                }
                (hir.placement(id)?.name.clone(), Some(Decl::Instance(id)))
            }
        };
        let Some(name) = name else {
            continue;
        };
        if variant_name(hir, vocabulary, variants, &name, report)? {
            let namespace = if in_prelude {
                &mut prelude
            } else {
                &mut source
            };
            namespace.duplicate(&name.text);
            continue;
        }
        if in_prelude {
            if let Some(Decl::Native(_)) = prelude.get(&name.text) {
                report.error(
                    hir,
                    Code::DuplicateName,
                    name.loc,
                    format!("`{}` is a native function of the vocabulary", name.text),
                );
                continue;
            }
            if declare(
                hir,
                &base,
                &mut prelude,
                &mut prelude_first,
                &name,
                decl,
                report,
            ) {
                match decl {
                    Some(decl) => {
                        source.insert(name.text.clone(), decl);
                    }
                    None => source.duplicate(&name.text),
                }
            }
        } else {
            declare(hir, &base, &mut source, &mut first, &name, decl, report);
        }
    }
    let (recursive, calls) = recursive_functions(hir, &source, &prelude)?;
    let inline = InlineCounts::of(hir, &source, &prelude, &recursive)?;
    let graph = PatternGraph::of(hir, &source, &prelude, report)?;
    let cell_groups = CellGroups::of(hir, &source)?;
    Ok(Collected {
        source,
        prelude,
        recursive,
        calls,
        inline,
        cyclic_patterns: graph.cyclic,
        stand_ins: graph.stand_ins,
        placeholder_patterns: graph.placeholders,
        cell_groups,
    })
}

fn base_namespace(vocabulary: &Vocabulary) -> Result<Namespace, Internal> {
    let mut base = Namespace::default();
    for (index, decl) in vocabulary.node_kinds().iter().enumerate() {
        let kind = vocabulary
            .node_kind_at(index)
            .ok_or(Internal::missed("node kind"))?;
        base.insert(decl.name.clone(), Decl::Node(kind));
    }
    for (index, decl) in vocabulary.wire_kinds().iter().enumerate() {
        let kind = vocabulary
            .wire_kind_at(index)
            .ok_or(Internal::missed("wire kind"))?;
        base.insert(decl.name.clone(), Decl::Wire(kind));
    }
    for (index, decl) in vocabulary.enums().iter().enumerate() {
        let id = vocabulary.enum_at(index).ok_or(Internal::missed("enum"))?;
        base.insert(decl.name.clone(), Decl::Enum(id));
    }
    for builtin in BuiltinName::all() {
        base.insert(builtin.name().to_string(), Decl::Builtin(builtin));
    }
    Ok(base)
}

fn declare(
    hir: &Hir,
    base: &Namespace,
    namespace: &mut Namespace,
    first: &mut BTreeMap<String, Loc>,
    name: &Ident,
    decl: Option<Decl>,
    report: &mut Report,
) -> bool {
    if let Some(existing) = base.get(&name.text) {
        let what = match existing {
            Decl::Builtin(_) => "a built-in name",
            Decl::Node(_) => "a node kind of the vocabulary",
            Decl::Wire(_) => "a wire kind of the vocabulary",
            Decl::Enum(_) => "an enum of the vocabulary",
            Decl::Input(_)
            | Decl::Let(_)
            | Decl::State(_)
            | Decl::Fn(_)
            | Decl::Native(_)
            | Decl::Pattern(_)
            | Decl::Instance(_) => "already declared",
        };
        namespace.duplicate(&name.text);
        report.error(
            hir,
            Code::DuplicateName,
            name.loc,
            format!("`{}` is {what}", name.text),
        );
        return false;
    }
    if let Some(earlier) = first.get(&name.text) {
        namespace.duplicate(&name.text);
        report.error_with(
            hir,
            Code::DuplicateName,
            name.loc,
            &[(*earlier, "first declared here".to_string())],
            format!("`{}` is declared twice", name.text),
        );
        return false;
    }
    first.insert(name.text.clone(), name.loc);
    match decl {
        Some(decl) => {
            namespace.insert(name.text.clone(), decl);
        }
        None => namespace.duplicate(&name.text),
    }
    true
}

fn variant_name(
    hir: &Hir,
    vocabulary: &Vocabulary,
    variants: &Variants,
    name: &Ident,
    report: &mut Report,
) -> Result<bool, Internal> {
    if let Some(id) = variants.owner(&name.text) {
        let owner = vocabulary
            .enum_decl(id)
            .ok_or(Internal::missed("enum"))?
            .name
            .as_str();
        report.error(
            hir,
            Code::VariantName,
            name.loc,
            format!(
                "`{}` is a variant of the enum `{owner}` and is never a declared name",
                name.text
            ),
        );
        return Ok(true);
    }
    Ok(false)
}
