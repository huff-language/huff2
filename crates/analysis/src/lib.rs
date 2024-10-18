pub mod errors;

use crate::errors::AnalysisError;
use huff_ast::{Definition, IdentifiableNode, Instruction, Invoke, Macro, MacroStatement};
use std::collections::{BTreeMap, BTreeSet};

pub fn analyze<
    'src,
    'ast: 'src,
    I: Iterator<Item = &'ast Definition<'src>>,
    E: FnMut(AnalysisError<'ast, 'src>) + Copy,
>(
    defs: I,
    mut emit_error: E,
) -> Option<BTreeMap<&'src str, &'ast Definition<'src>>> {
    let global_defs = build_ident_map(defs);

    let mut analyzed_macros: BTreeSet<&str> = BTreeSet::new();
    let mut invoke_stack: Vec<(&'ast Macro<'src>, &'ast Invoke<'src>)> = Vec::with_capacity(32);

    global_defs.iter().for_each(|(_, defs)| {
        defs.iter()
            .filter_map(|def| {
                if let Definition::Macro(m) = def {
                    Some(m)
                } else {
                    None
                }
            })
            .for_each(|m| {
                analyze_macro(
                    &global_defs,
                    m,
                    &mut invoke_stack,
                    &mut analyzed_macros,
                    emit_error,
                );
                analyzed_macros.insert(m.ident());
            })
    });

    global_defs
        .into_iter()
        .try_fold(BTreeMap::new(), |mut unique, (name, found_defs)| {
            let (name, def) = match found_defs.as_slice() {
                &[sole_def] => Some((name, sole_def)),
                [] => None,
                many_defs => {
                    emit_error(AnalysisError::DefinitionNameCollision {
                        collided: many_defs.to_vec().into_boxed_slice(),
                        duplicate_name: name,
                    });
                    None
                }
            }?;
            unique.insert(name, def);
            Some(unique)
        })
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

fn analyze_macro<'ast: 'src, 'src, E: FnMut(AnalysisError<'ast, 'src>) + Copy>(
    global_defs: &BTreeMap<&'src str, Vec<&'ast Definition<'src>>>,
    m: &'ast Macro<'src>,
    invoke_stack: &mut Vec<(&'ast Macro<'src>, &'ast Invoke<'src>)>,
    validated_macros: &mut BTreeSet<&'src str>,
    mut emit_error: E,
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
        MacroStatement::LabelDefinition(label_name) => Some(label_name),
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
    let analyze_instruction = |instruction: &'ast Instruction| match instruction {
        Instruction::Op(_) => None,
        Instruction::LabelReference(label) => {
            if !in_ident_map(&labels, label.ident()) {
                Some(AnalysisError::ReferenceNotFound {
                    scope: m,
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
                    not_found: instruction,
                })
            } else {
                None
            }
        }
        Instruction::ConstantReference(const_ref) => {
            if !global_exists!(global_defs, const_ref.ident(), Definition::Constant { .. }) {
                Some(AnalysisError::ReferenceNotFound {
                    scope: m,
                    not_found: instruction,
                })
            } else {
                None
            }
        }
    };

    m.body.iter().for_each(|stmt| match stmt {
        MacroStatement::LabelDefinition(_) => {}
        MacroStatement::Instruction(instruction) => {
            analyze_instruction(instruction).map(emit_error);
        }
        MacroStatement::Invoke(invoke) => match invoke {
            Invoke::Macro { name, args } => {
                // Check the arguments in the invocatino.
                args.iter()
                    .filter_map(analyze_instruction)
                    .for_each(emit_error);
                // Emit error if we don't find at least 1 macro by the given name.
                if !global_exists!(global_defs, name.ident(), Definition::Macro(_)) {
                    emit_error(AnalysisError::MacroNotFound { scope: m, name });
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
                                scope: m,
                                args,
                                target: macro_being_invoked,
                            });
                        }
                        analyze_macro(
                            global_defs,
                            macro_being_invoked,
                            invoke_stack,
                            validated_macros,
                            emit_error,
                        );
                    });
                invoke_stack.pop().unwrap();

                validated_macros.insert(name.ident());
            }
            _ => todo!(),
        },
    });
}

fn build_ident_map<'ast, 'src, N: IdentifiableNode<'src>, I: Iterator<Item = &'ast N>>(
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
