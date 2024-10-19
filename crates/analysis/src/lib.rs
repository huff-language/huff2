pub mod errors;
pub mod label_stack;

use crate::errors::AnalysisError;
use crate::label_stack::LabelStack;
use huff_ast::{Definition, IdentifiableNode, Instruction, Invoke, Macro, MacroStatement};
use std::collections::{BTreeMap, BTreeSet};

pub fn analyze_global_for_dups<'src, 'ast: 'src, E: FnMut(AnalysisError<'ast, 'src>)>(
    global_defs: &BTreeMap<&'src str, Vec<&'ast Definition<'src>>>,
    mut emit_error: E,
) -> BTreeMap<&'src str, &'ast Definition<'src>> {
    global_defs
        .iter()
        .filter_map(|(name, found_defs)| match found_defs.as_slice() {
            [] => None,
            [found_def] => Some((*name, *found_def)),
            many_defs => {
                emit_error(AnalysisError::DefinitionNameCollision {
                    collided: many_defs.to_vec().into_boxed_slice(),
                    duplicate_name: name,
                });
                None
            }
        })
        .collect()
}

pub fn analyze_entry_point<'src, 'ast: 'src, E: FnMut(AnalysisError<'ast, 'src>)>(
    global_defs: &BTreeMap<&'src str, Vec<&'ast Definition<'src>>>,
    entry_point: &'ast Macro<'src>,
    mut emit_error: E,
) {
    let mut analyzed_macros: BTreeSet<&str> = BTreeSet::new();
    let mut invoke_stack: Vec<(&'ast Macro<'src>, &'ast Invoke<'src>)> = Vec::with_capacity(32);
    let mut label_stack = LabelStack::default();

    if entry_point.args.len() != 0 {
        emit_error(AnalysisError::MacroArgumentCountMismatch {
            scope: None,
            args: &[],
            target: entry_point,
        });
    }

    analyze_macro(
        global_defs,
        entry_point,
        &mut label_stack,
        &mut invoke_stack,
        &mut analyzed_macros,
        &mut emit_error,
    );
}

macro_rules! global_exists {
    ($global_defs:expr, $ident:expr, $pattern:pat) => {
        $global_defs
            .get($ident)
            .map(|defs| {
                defs.iter().any(|def| match def {
                    $pattern => true,
                    _ => false,
                })
            })
            .unwrap_or(false)
    };
}

fn analyze_macro<'ast: 'src, 'src, E: FnMut(AnalysisError<'ast, 'src>)>(
    global_defs: &BTreeMap<&'src str, Vec<&'ast Definition<'src>>>,
    m: &'ast Macro<'src>,
    label_stack: &mut LabelStack<'src, ()>,
    invoke_stack: &mut Vec<(&'ast Macro<'src>, &'ast Invoke<'src>)>,
    validated_macros: &mut BTreeSet<&'src str>,
    emit_error: &mut E,
) {
    let name = m.name.0;

    // If we already validated this macro, return.
    if validated_macros.contains(name) {
        return;
    }

    if invoke_stack
        .iter()
        .any(|(invoked, _)| invoked.name.0 == name)
    {
        emit_error(AnalysisError::RecursiveMacroInvocation {
            invocation_chain: invoke_stack.clone().into_boxed_slice(),
        });
        return;
    }

    let labels = build_ident_map(m.body.iter().filter_map(|stmt| match stmt {
        MacroStatement::LabelDefinition(label_name) => {
            label_stack.add(label_name.ident());
            Some(label_name)
        }
        _ => None,
    }));

    let macro_args = build_ident_map(m.args.iter());

    labels.iter().for_each(|(_name, defs)| {
        if defs.len() >= 2 {
            emit_error(AnalysisError::DuplicateLabelDefinition {
                scope: m,
                duplicates: defs.clone().into_boxed_slice(),
            })
        }
    });

    // Validate instruction against the current scope.
    let analyze_instruction =
        |instruction: &'ast Instruction, label_stack: &mut LabelStack<'src, ()>| match instruction {
            Instruction::Op(_) | Instruction::VariablePush(_) => None,
            Instruction::LabelReference(label) => {
                if !label_stack.contains(label.ident()) {
                    Some(AnalysisError::ReferenceNotFound {
                        scope: m,
                        ref_type: "label",
                        not_found: instruction,
                    })
                } else {
                    None
                }
            }
            Instruction::MacroArgReference(arg) => {
                if !in_ident_map(&macro_args, arg.ident()) {
                    Some(AnalysisError::ReferenceNotFound {
                        scope: m,
                        ref_type: "macro argument",
                        not_found: instruction,
                    })
                } else {
                    None
                }
            }
            Instruction::ConstantReference(const_ref) => {
                if !global_exists!(global_defs, const_ref.ident(), Definition::Constant { .. }) {
                    Some(AnalysisError::DefinitionNotFound {
                        scope: m,
                        def_type: "constant",
                        name: const_ref,
                    })
                } else {
                    None
                }
            }
        };

    label_stack.enter_context();

    m.body.iter().for_each(|stmt| match stmt {
        MacroStatement::LabelDefinition(_) => {}
        MacroStatement::Instruction(instruction) => {
            if let Some(err) = analyze_instruction(instruction, label_stack) {
                emit_error(err);
            }
        }
        MacroStatement::Invoke(invoke) => match invoke {
            Invoke::Macro { name, args } => {
                // Check the arguments in the invocatino.
                // Not actually redundant so making clippy stfu here.
                #[allow(clippy::redundant_closure)]
                args.iter()
                    .filter_map(|arg| analyze_instruction(arg, label_stack))
                    .for_each(|err| emit_error(err));
                // Emit error if we don't find at least 1 macro by the given name.
                if !global_exists!(global_defs, name.ident(), Definition::Macro(_)) {
                    emit_error(AnalysisError::DefinitionNotFound {
                        scope: m,
                        def_type: "macro",
                        name,
                    });
                }
                invoke_stack.push((m, invoke));

                // Filter and process all macros with given name to make sure errors are complete.
                global_defs
                    .get(name.ident())
                    .map(|found| found.as_slice())
                    .unwrap_or(&[])
                    .iter()
                    .filter_map(|def| {
                        if let Definition::Macro(macro_being_invoked) = def {
                            Some(macro_being_invoked)
                        } else {
                            None
                        }
                    })
                    .for_each(|macro_being_invoked| {
                        if macro_being_invoked.args.len() != args.len() {
                            emit_error(AnalysisError::MacroArgumentCountMismatch {
                                scope: Some(m),
                                args,
                                target: macro_being_invoked,
                            });
                        }
                        analyze_macro(
                            global_defs,
                            macro_being_invoked,
                            label_stack,
                            invoke_stack,
                            validated_macros,
                            emit_error,
                        );
                    });
                invoke_stack.pop().unwrap();

                validated_macros.insert(name.ident());
            }
            Invoke::BuiltinTableSize(table_ref) | Invoke::BuiltinTableStart(table_ref) => {
                if !global_exists!(
                    global_defs,
                    table_ref.ident(),
                    Definition::Table { .. } | Definition::Jumptable(_)
                ) {
                    emit_error(AnalysisError::DefinitionNotFound {
                        scope: m,
                        def_type: "table",
                        name: table_ref,
                    })
                }
            }
            Invoke::BuiltinCodeSize(code_ref) | Invoke::BuiltinCodeOffset(code_ref) => {
                if !global_exists!(global_defs, code_ref.ident(), Definition::Macro(_)) {
                    emit_error(AnalysisError::DefinitionNotFound {
                        scope: m,
                        def_type: "macro",
                        name: code_ref,
                    })
                }
                if global_defs
                    .get(code_ref.ident())
                    .map(|defs| {
                        defs.iter()
                            .any(|def| matches!(def, Definition::Macro(m) if m.args.len() > 0))
                    })
                    .unwrap_or(false)
                {
                    emit_error(AnalysisError::NotYetSupported {
                        intent: "code introspection for macros with arguments".to_owned(),
                        span: ((), code_ref.1),
                    })
                }
            }
            Invoke::BuiltinFuncSig(func_or_error_ref) | Invoke::BuiltinError(func_or_error_ref) => {
                if !global_exists!(
                    global_defs,
                    func_or_error_ref.ident(),
                    Definition::SolFunction(_) | Definition::SolError(_)
                ) {
                    emit_error(AnalysisError::DefinitionNotFound {
                        scope: m,
                        def_type: "solidity function / error",
                        name: func_or_error_ref,
                    })
                }
            }
            Invoke::BuiltinEventHash(event_ref) => {
                if !global_exists!(global_defs, event_ref.ident(), Definition::SolEvent(_)) {
                    emit_error(AnalysisError::DefinitionNotFound {
                        scope: m,
                        def_type: "solidity event",
                        name: event_ref,
                    })
                }
            }
        },
    });

    label_stack.leave_context();
}

pub fn build_ident_map<'ast, 'src, N: IdentifiableNode<'src>, I: Iterator<Item = &'ast N>>(
    nodes: I,
) -> BTreeMap<&'src str, Vec<&'ast N>> {
    let mut ident_map: BTreeMap<&'src str, Vec<&'ast N>> = BTreeMap::new();
    nodes.for_each(|node| {
        ident_map
            .entry(node.ident())
            .or_insert_with(|| Vec::with_capacity(1))
            .push(node)
    });
    ident_map
}

fn in_ident_map<'ast, 'src, N: IdentifiableNode<'src>>(
    ident_map: &BTreeMap<&'src str, Vec<&'ast N>>,
    ident: &'src str,
) -> bool {
    ident_map.get(ident).map(|found| found.len()).unwrap_or(0) > 0
}

#[cfg(test)]
mod test {
    use super::*;
    use chumsky::prelude::*;
    use huff_ast::*;

    fn emits_analysis_error<'defs: 'src, 'src, const M: usize, const N: usize>(
        defs: [&'defs Definition<'src>; M],
        entry_point: &'defs Macro<'src>,
        errors: [AnalysisError<'_, 'src>; N],
    ) {
        let mut emitted = Vec::with_capacity(N);
        analyze_entry_point(&build_ident_map(defs.into_iter()), entry_point, |err| {
            emitted.push(err.clone())
        });
        assert_eq!(errors.to_vec(), emitted, "expected == emitted");
    }

    #[test]
    fn duplicate_macro_definition() {
        let span = SimpleSpan::new(0, 0);
        let d1 = Definition::Macro(Macro {
            name: ("Thing", span),
            args: Box::new([]),
            takes_returns: None,
            body: Box::new([]),
        });
        let d2 = Definition::Macro(Macro {
            name: ("Thing", span),
            args: Box::new([("wow", span)]),
            takes_returns: None,
            body: Box::new(
                [MacroStatement::Instruction(Instruction::MacroArgReference(("wow", span)))],
            ),
        });

        let mut emitted = vec![];
        let map = analyze_global_for_dups(&build_ident_map([&d1, &d2].into_iter()), |err| {
            emitted.push(err)
        });
        assert_eq!(
            emitted,
            vec![AnalysisError::DefinitionNameCollision {
                collided: Box::new([&d1, &d2]),
                duplicate_name: "Thing"
            }]
        );
        assert_eq!(map, BTreeMap::new());
    }

    #[test]
    fn more_than_two_deplicate_defs() {
        let span = SimpleSpan::new(0, 0);
        let d1 = Definition::Macro(Macro {
            name: ("TheWhat", span),
            args: Box::new([]),
            takes_returns: None,
            body: Box::new([]),
        });
        let d2 = Definition::Constant {
            name: ("TheWhat", span),
            expr: (ConstExpr::FreeStoragePointer, span),
        };

        let unrelated_table = Definition::Table {
            name: ("awesome_stuff", span),
            data: Box::new([0x00, 0x01]),
        };
        let d3 = Definition::Macro(Macro {
            name: ("TheWhat", span),
            args: Box::new([("nice", span)]),
            takes_returns: None,
            body: Box::new([]),
        });

        let mut emitted = vec![];
        let map = analyze_global_for_dups(
            &build_ident_map([&d1, &d2, &unrelated_table, &d3].into_iter()),
            |err| emitted.push(err),
        );
        assert_eq!(
            emitted,
            vec![AnalysisError::DefinitionNameCollision {
                collided: Box::new([&d1, &d2, &d3]),
                duplicate_name: "TheWhat"
            }]
        );
        assert_eq!(map, BTreeMap::from([("awesome_stuff", &unrelated_table)]));
    }

    #[test]
    fn simple_recursive_macro_invoke() {
        let span = SimpleSpan::new(0, 0);
        let invoke = Invoke::Macro {
            name: ("TheRizzler", span),
            args: Box::new([]),
        };
        let inner_macro = Macro {
            name: ("TheRizzler", span),
            args: Box::new([]),
            takes_returns: None,
            body: Box::new([MacroStatement::Invoke(invoke.clone())]),
        };
        let m = Definition::Macro(inner_macro.clone());
        emits_analysis_error(
            [&m],
            &inner_macro,
            [AnalysisError::RecursiveMacroInvocation {
                invocation_chain: Box::new([(&inner_macro, &invoke)]),
            }],
        );
    }

    #[test]
    fn deep_recursive_macro_invoke() {
        let span = SimpleSpan::new(0, 0);

        let invoke1 = Invoke::Macro {
            name: ("Top", span),
            args: Box::new([]),
        };
        let inner_m1 = Macro {
            name: ("VeryTop", span),
            args: Box::new([]),
            takes_returns: None,
            body: Box::new([MacroStatement::Invoke(invoke1.clone())]),
        };
        let m1 = Definition::Macro(inner_m1.clone());

        let invoke2 = Invoke::Macro {
            name: ("Lower", span),
            args: Box::new([]),
        };
        let inner_m2 = Macro {
            name: ("Top", span),
            args: Box::new([]),
            takes_returns: None,
            body: Box::new([MacroStatement::Invoke(invoke2.clone())]),
        };
        let m2 = Definition::Macro(inner_m2.clone());

        let invoke3 = Invoke::Macro {
            name: ("VeryTop", span),
            args: Box::new([]),
        };
        let inner_m3 = Macro {
            name: ("Lower", span),
            args: Box::new([]),
            takes_returns: None,
            body: Box::new([MacroStatement::Invoke(invoke3.clone())]),
        };
        let m3 = Definition::Macro(inner_m3.clone());

        emits_analysis_error(
            [&m1, &m2, &m3],
            &inner_m1,
            [AnalysisError::RecursiveMacroInvocation {
                invocation_chain: Box::new([
                    (&inner_m1, &invoke1),
                    (&inner_m2, &invoke2),
                    (&inner_m3, &invoke3),
                ]),
            }],
        );
    }

    #[test]
    fn macro_not_found() {
        let span = SimpleSpan::new(0, 0);
        let invoke_span = SimpleSpan::new(3, 12);

        let invoke = Invoke::Macro {
            name: ("MY_FUNC", invoke_span),
            args: Box::new([]),
        };
        let inner_macro = Macro {
            name: ("MAIN", span),
            args: Box::new([]),
            takes_returns: None,
            body: Box::new([MacroStatement::Invoke(invoke.clone())]),
        };
        let m = Definition::Macro(inner_macro.clone());

        emits_analysis_error(
            [&m],
            &inner_macro,
            [AnalysisError::DefinitionNotFound {
                scope: &inner_macro,
                def_type: "macro",
                name: &("MY_FUNC", invoke_span),
            }],
        );
    }

    #[test]
    fn nested_label_ref_valid() {
        let span = SimpleSpan::new(0, 0);

        let im1 = Macro {
            name: ("MAIN", span),
            args: Box::new([]),
            takes_returns: None,
            body: Box::new([
                MacroStatement::LabelDefinition(("wow", span)),
                MacroStatement::Invoke(Invoke::Macro {
                    name: ("OTHER", span),
                    args: Box::new([]),
                }),
            ]),
        };
        let m1 = Definition::Macro(im1.clone());

        let im2 = Macro {
            name: ("OTHER", span),
            args: Box::new([]),
            takes_returns: None,
            body: Box::new([MacroStatement::Instruction(Instruction::LabelReference((
                "wow", span,
            )))]),
        };
        let m2 = Definition::Macro(im2.clone());

        emits_analysis_error([&m1, &m2], &im1, []);
    }
}
