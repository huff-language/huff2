pub mod errors;
pub mod label_stack;

use crate::errors::AnalysisError;
use crate::label_stack::LabelStack;
use huff_ast::{Definition, IdentifiableNode, Instruction, Invoke, Macro, MacroStatement, Spanned};
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
    entry_point_name: &'src str,
    mut emit_error: E,
) -> Option<&'ast Macro<'src>> {
    let mut analyzed_macros: BTreeSet<&str> = BTreeSet::new();
    let mut invoke_stack = Vec::with_capacity(32);
    let mut label_stack = LabelStack::default();

    let entry_point = if let Some(Definition::Macro(entry_point)) = global_defs
        .get(entry_point_name)
        .and_then(|defs| defs.first())
    {
        entry_point
    } else {
        emit_error(AnalysisError::EntryPointNotFound {
            name: entry_point_name,
        });
        return None;
    };

    if entry_point.args.0.len() != 0 {
        emit_error(AnalysisError::MacroArgumentCountMismatch {
            scope: None,
            args: &[],
            target: entry_point,
        });
    }

    MacroAnalysis::run(
        global_defs,
        entry_point,
        &mut label_stack,
        &mut invoke_stack,
        &mut analyzed_macros,
        &mut emit_error,
    );

    Some(entry_point)
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

struct MacroAnalysis<'a, 'src, 'ast: 'src, E: FnMut(AnalysisError<'ast, 'src>)> {
    global_defs: &'a BTreeMap<&'src str, Vec<&'ast Definition<'src>>>,
    m: &'ast Macro<'src>,
    label_stack: &'a mut LabelStack<'src, ()>,
    invoke_stack: &'a mut Vec<(&'ast Macro<'src>, &'ast Spanned<&'src str>)>,
    validated_macros: &'a mut BTreeSet<&'src str>,
    emit_error: &'a mut E,
}

impl<'a, 'src, 'ast: 'src, E: FnMut(AnalysisError<'ast, 'src>)> MacroAnalysis<'a, 'src, 'ast, E> {
    fn emit(&mut self, err: AnalysisError<'ast, 'src>) {
        (self.emit_error)(err);
    }

    fn run(
        global_defs: &'a BTreeMap<&'src str, Vec<&'ast Definition<'src>>>,
        m: &'ast Macro<'src>,
        label_stack: &'a mut LabelStack<'src, ()>,
        invoke_stack: &'a mut Vec<(&'ast Macro<'src>, &'ast Spanned<&'src str>)>,
        validated_macros: &'a mut BTreeSet<&'src str>,
        emit_error: &'a mut E,
    ) {
        MacroAnalysis {
            global_defs,
            m,
            label_stack,
            invoke_stack,
            validated_macros,
            emit_error,
        }
        .analyze();
    }

    fn analyze(&mut self) {
        let name = self.m.name.0;

        // If we already validated this macro, return.
        if self.validated_macros.contains(name) {
            return;
        }

        if self
            .invoke_stack
            .iter()
            .any(|(invoked, _)| invoked.name.0 == name)
        {
            self.emit(AnalysisError::RecursiveMacroInvocation {
                invocation_chain: self.invoke_stack.clone().into_boxed_slice(),
            });
            return;
        }

        let labels = build_ident_map(self.m.body.iter().filter_map(|stmt| match stmt {
            MacroStatement::LabelDefinition(label_name) => {
                self.label_stack.add(label_name.ident());
                Some(label_name)
            }
            _ => None,
        }));

        let macro_args = build_ident_map(self.m.args.0.iter());

        labels.iter().for_each(|(_name, defs)| {
            if defs.len() >= 2 {
                self.emit(AnalysisError::DuplicateLabelDefinition {
                    scope: self.m,
                    duplicates: defs.clone().into_boxed_slice(),
                })
            }
        });

        self.label_stack.enter_context();

        self.m.body.iter().for_each(|stmt| match stmt {
            MacroStatement::LabelDefinition(_) => {}
            MacroStatement::Instruction(instruction) => {
                self.analyze_instruction(&macro_args, instruction);
            }
            MacroStatement::Invoke(invoke) => match invoke {
                Invoke::Macro { name, args } => {
                    // Check the arguments in the invocatino.
                    // Not actually redundant so making clippy stfu here.
                    #[allow(clippy::redundant_closure)]
                    args.iter()
                        .for_each(|arg| self.analyze_instruction(&macro_args, arg));
                    // Emit error if we don't find at least 1 macro by the given name.
                    if !global_exists!(self.global_defs, name.ident(), Definition::Macro(_)) {
                        self.emit(AnalysisError::DefinitionNotFound {
                            scope: self.m,
                            def_type: "macro",
                            name,
                        });
                    }
                    self.invoke_stack.push((self.m, name));

                    // Filter and process all macros with given name to make sure errors are complete.
                    self.global_defs
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
                            if macro_being_invoked.args.0.len() != args.len() {
                                self.emit(AnalysisError::MacroArgumentCountMismatch {
                                    scope: Some(self.m),
                                    args,
                                    target: macro_being_invoked,
                                });
                            }
                            MacroAnalysis::run(
                                self.global_defs,
                                macro_being_invoked,
                                self.label_stack,
                                self.invoke_stack,
                                self.validated_macros,
                                self.emit_error,
                            );
                        });
                    self.invoke_stack.pop().unwrap();

                    if name.ident() != self.m.ident() {
                        self.validated_macros.insert(name.ident());
                    }
                }
                Invoke::BuiltinTableSize(table_ref) | Invoke::BuiltinTableStart(table_ref) => {
                    if !global_exists!(
                        self.global_defs,
                        table_ref.ident(),
                        Definition::Table { .. } | Definition::Jumptable(_)
                    ) {
                        self.emit(AnalysisError::DefinitionNotFound {
                            scope: self.m,
                            def_type: "table",
                            name: table_ref,
                        })
                    }
                }
                Invoke::BuiltinCodeSize(code_ref) | Invoke::BuiltinCodeOffset(code_ref) => {
                    if !global_exists!(self.global_defs, code_ref.ident(), Definition::Macro(_)) {
                        self.emit(AnalysisError::DefinitionNotFound {
                            scope: self.m,
                            def_type: "macro",
                            name: code_ref,
                        })
                    }
                    if self
                        .global_defs
                        .get(code_ref.ident())
                        .map(|defs| {
                            defs.iter().any(
                                |def| matches!(def, Definition::Macro(m) if m.args.0.len() > 0),
                            )
                        })
                        .unwrap_or(false)
                    {
                        self.emit(AnalysisError::NotYetSupported {
                            intent: "code introspection for macros with arguments".to_owned(),
                            span: ((), code_ref.1),
                        })
                    }
                }
                Invoke::BuiltinFuncSig(func_or_error_ref)
                | Invoke::BuiltinError(func_or_error_ref) => {
                    if !global_exists!(
                        self.global_defs,
                        func_or_error_ref.ident(),
                        Definition::SolFunction(_) | Definition::SolError(_)
                    ) {
                        self.emit(AnalysisError::DefinitionNotFound {
                            scope: self.m,
                            def_type: "solidity function / error",
                            name: func_or_error_ref,
                        })
                    }
                }
                Invoke::BuiltinEventHash(event_ref) => {
                    if !global_exists!(self.global_defs, event_ref.ident(), Definition::SolEvent(_))
                    {
                        self.emit(AnalysisError::DefinitionNotFound {
                            scope: self.m,
                            def_type: "solidity event",
                            name: event_ref,
                        })
                    }
                }
            },
        });

        self.label_stack.leave_context();
    }

    fn analyze_instruction(
        &mut self,
        macro_args: &BTreeMap<&str, Vec<&Spanned<&str>>>,
        instruction: &'ast Instruction<'src>,
    ) {
        match instruction {
            Instruction::LabelReference(label) => {
                if !self.label_stack.contains(label.ident()) {
                    self.emit(AnalysisError::LabelNotFound {
                        scope: self.m,
                        invocation_chain: self.invoke_stack.clone().into_boxed_slice(),
                        not_found: label,
                    })
                }
            }
            Instruction::MacroArgReference(arg) => {
                if !in_ident_map(macro_args, arg.ident()) {
                    self.emit(AnalysisError::MacroArgNotFound {
                        scope: self.m,
                        not_found: arg,
                    })
                }
            }
            Instruction::ConstantReference(const_ref) => {
                if !global_exists!(
                    self.global_defs,
                    const_ref.ident(),
                    Definition::Constant { .. }
                ) {
                    self.emit(AnalysisError::DefinitionNotFound {
                        scope: self.m,
                        def_type: "constant",
                        name: const_ref,
                    });
                }
            }

            Instruction::Op(_) | Instruction::VariablePush(_) => {}
        }
    }
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
        entry_point_name: &'src str,
        errors: [AnalysisError<'_, 'src>; N],
    ) {
        let mut emitted = Vec::with_capacity(N);
        analyze_entry_point(
            &build_ident_map(defs.into_iter()),
            entry_point_name,
            |err| emitted.push(err.clone()),
        );
        assert_eq!(errors.to_vec(), emitted, "expected == emitted");
    }

    #[test]
    fn duplicate_macro_definition() {
        let span = SimpleSpan::new(0, 0);
        let d1 = Definition::Macro(Macro {
            name: ("Thing", span),
            args: (Box::new([]), span),
            takes_returns: None,
            body: Box::new([]),
        });
        let d2 = Definition::Macro(Macro {
            name: ("Thing", span),
            args: (Box::new([("wow", span)]), span),
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
            args: (Box::new([]), span),
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
            args: (Box::new([("nice", span)]), span),
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
            args: (Box::new([]), span),
            takes_returns: None,
            body: Box::new([MacroStatement::Invoke(invoke.clone())]),
        };
        let m = Definition::Macro(inner_macro.clone());
        emits_analysis_error(
            [&m],
            "TheRizzler",
            [AnalysisError::RecursiveMacroInvocation {
                invocation_chain: Box::new([(&inner_macro, &("TheRizzler", span))]),
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
            args: (Box::new([]), span),
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
            args: (Box::new([]), span),
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
            args: (Box::new([]), span),
            takes_returns: None,
            body: Box::new([MacroStatement::Invoke(invoke3.clone())]),
        };
        let m3 = Definition::Macro(inner_m3.clone());

        emits_analysis_error(
            [&m1, &m2, &m3],
            "VeryTop",
            [AnalysisError::RecursiveMacroInvocation {
                invocation_chain: Box::new([
                    (&inner_m1, &("Top", span)),
                    (&inner_m2, &("Lower", span)),
                    (&inner_m3, &("VeryTop", span)),
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
            args: (Box::new([]), span),
            takes_returns: None,
            body: Box::new([MacroStatement::Invoke(invoke.clone())]),
        };
        let m = Definition::Macro(inner_macro.clone());

        emits_analysis_error(
            [&m],
            "MAIN",
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
            args: (Box::new([]), span),
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
            args: (Box::new([]), span),
            takes_returns: None,
            body: Box::new([MacroStatement::Instruction(Instruction::LabelReference((
                "wow", span,
            )))]),
        };
        let m2 = Definition::Macro(im2.clone());

        emits_analysis_error([&m1, &m2], "MAIN", []);
    }
}
