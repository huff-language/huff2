use ariadne::{Color, Config, Fmt, IndexType, Label, Report, ReportKind};
use huff_ast::{Definition, IdentifiableNode, Instruction, Macro, Span, Spanned, Text};

type InvokeChain<'src, 'ast> = Box<[(&'ast Text<'src>, &'ast Text<'src>)]>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inclusion<'src, 'ast: 'src> {
    pub entry_point: Text<'src>,
    pub invoke_stack: InvokeChain<'src, 'ast>,
    pub inclusion: Spanned<&'src str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisError<'ast, 'src> {
    /// When two different definitions have the same.
    DefinitionNameCollision {
        collided: Box<[&'ast Definition<'src>]>,
        duplicate_name: &'src str,
    },
    RecursiveMacroInvocation {
        invocation_chain: InvokeChain<'src, 'ast>,
    },
    RecursiveCodeInclusion {
        linking_inclusions: Box<[Inclusion<'src, 'ast>]>,
    },
    MacroLabelNotFound {
        scope: &'ast Text<'src>,
        invocation_chain: InvokeChain<'src, 'ast>,
        not_found: &'ast Text<'src>,
    },
    TableLabelNotFound {
        scope: &'ast Text<'src>,
        table_ref: &'ast Text<'src>,
        table_def: &'ast Text<'src>,
        not_found: &'ast Text<'src>,
    },
    MacroArgNotFound {
        scope: &'ast Macro<'src>,
        not_found: &'ast Text<'src>,
    },
    EntryPointNotFound {
        name: &'src str,
    },
    DefinitionNotFound {
        def_type: &'static str,
        not_found: &'ast Text<'src>,
    },
    EntryPointHasArgs {
        target: &'ast Macro<'src>,
    },
    MacroArgumentCountMismatch {
        scope: &'ast Macro<'src>,
        invoke: &'ast Text<'src>,
        args: &'ast Spanned<Box<[Instruction<'src>]>>,
        target: &'ast Macro<'src>,
    },
    DuplicateLabelDefinition {
        scope: &'ast Macro<'src>,
        duplicates: Box<[&'ast Text<'src>]>,
    },
    DuplicateMacroArgDefinition {
        scope: &'ast Macro<'src>,
        duplicates: Box<[&'ast Text<'src>]>,
    },
    NotYetSupported {
        intent: String,
        span: Span,
    },
}

impl AnalysisError<'_, '_> {
    pub fn report(&self, filename: String) -> Report<(String, std::ops::Range<usize>)> {
        match self {
            Self::DefinitionNameCollision {
                collided,
                duplicate_name,
            } => {
                let first_span = collided
                    .iter()
                    .map(|def| def.spanned())
                    .max_by_key(|name| name.1.start)
                    .unwrap();

                let base_report =
                    Report::build(ReportKind::Error, filename.clone(), first_span.1.start)
                        .with_config(Config::default().with_index_type(IndexType::Byte))
                        .with_message(format!(
                            "Definitions with duplicate name '{}'",
                            duplicate_name.fg(Color::Red)
                        ));

                base_report
                    .with_labels(collided.iter().map(|def| {
                        Label::new((filename.clone(), def.spanned().1.into_range()))
                            .with_color(Color::Red)
                    }))
                    .with_help(format!(
                        "Change the names of the duplicate {}",
                        "definitions so that they're no longer equal."
                    ))
                    .finish()
            }
            Self::EntryPointNotFound { name } => {
                Report::build(ReportKind::Error, filename.clone(), 0)
                    .with_message(format!("Entry point '{}' not found", name.fg(Color::Red)))
                    .with_help(format!(
                        "Define the '{}' entry point or pick an alternative one via the {}",
                        name, "--alt-main/--alt-constructor CLI flags"
                    ))
                    .finish()
            }
            Self::RecursiveMacroInvocation { invocation_chain } => {
                let first_invoke = invocation_chain.first().unwrap();

                let base_report =
                    Report::build(ReportKind::Error, filename.clone(), first_invoke.1 .1.start)
                        .with_config(Config::default().with_index_type(IndexType::Byte))
                        .with_message(format!(
                            "Cannot expand macro {} with recursive dependency on itself",
                            first_invoke.0.ident().fg(Color::Red)
                        ));

                invocation_chain
                    .iter()
                    .enumerate()
                    .map(|(i, (scope, invoking))| {
                        (i == invocation_chain.len() - 1, scope, invoking)
                    })
                    .fold(base_report, |report, (is_last, scope, invoking)| {
                        let report = report.with_label(
                            Label::new((filename.clone(), scope.1.into_range()))
                                .with_color(Color::Red),
                        );

                        if is_last {
                            report.with_label(
                                Label::new((filename.clone(), invoking.1.into_range()))
                                    .with_color(Color::Yellow)
                                    .with_message(format!(
                                        "Which calls back into {}",
                                        first_invoke.0.ident().fg(Color::Red)
                                    )),
                            )
                        } else {
                            report.with_label(
                                Label::new((filename.clone(), invoking.1.into_range()))
                                    .with_color(Color::Yellow),
                            )
                        }
                    })
                    .with_help(format!(
                        "If you'd like to reuse some component of your code wrap it in a{}{}",
                        " separate macro and use that, alternatively if you need",
                        " recursion/repetition unwrap your logic into a system of jumps & labels."
                    ))
                    .finish()
            }
            Self::MacroArgNotFound { scope, not_found } => {
                Report::build(ReportKind::Error, filename.clone(), not_found.1.start)
                    .with_config(Config::default().with_index_type(IndexType::Byte))
                    .with_message(format!(
                        "Reference to {} '{}' not found in macro {}",
                        "macro argument".fg(Color::Cyan),
                        not_found.0.fg(Color::Red),
                        scope.ident().fg(Color::Blue)
                    ))
                    .with_label(
                        Label::new((filename.clone(), scope.name.1.into_range()))
                            .with_color(Color::Blue),
                    )
                    .with_label(
                        Label::new((filename.clone(), scope.args.1.into_range()))
                            .with_color(Color::Yellow),
                    )
                    .with_label(
                        Label::new((filename.clone(), not_found.1.into_range()))
                            .with_color(Color::Red),
                    )
                    .with_label(if scope.args.0.is_empty() {
                        Label::new((filename.clone(), scope.args.1.into_range()))
                            .with_color(Color::Red)
                            .with_message("no arguments")
                    } else {
                        let args_list_span = scope.args.1.start + 1..scope.args.1.end - 1;
                        Label::new((filename.clone(), args_list_span))
                            .with_color(Color::Red)
                            .with_message(format!(
                                "No '{}' in arguments list",
                                not_found.ident().fg(Color::Red)
                            ))
                    })
                    .with_help(
                        "Ensure the argument exists and is correct (names are case-sensitive)",
                    )
                    .finish()
            }
            Self::DefinitionNotFound {
                def_type,
                not_found,
            } => Report::build(ReportKind::Error, filename.clone(), not_found.1.start)
                .with_config(Config::default().with_index_type(IndexType::Byte))
                .with_message(format!(
                    "Definition of {} '{}' not found ",
                    def_type.fg(Color::Cyan),
                    not_found.0.fg(Color::Red),
                ))
                .with_label(
                    Label::new((filename.clone(), not_found.1.into_range())).with_color(Color::Red),
                )
                .finish(),
            Self::MacroLabelNotFound {
                scope,
                invocation_chain,
                not_found,
            } => {
                Report::build(ReportKind::Error, filename.clone(), not_found.1.start)
                    .with_config(Config::default().with_index_type(IndexType::Byte))
                    .with_message(format!(
                        "Label '{}' not found in macro {} or its parent contexts",
                        not_found.ident().fg(Color::Red),
                        scope.ident().fg(Color::Blue)
                    ))
                    .with_labels(invocation_chain.iter().rev().flat_map(
                        |(parent_scope, invoke)| {
                            [
                                Label::new((filename.clone(), parent_scope.span().into_range()))
                                    .with_color(Color::Yellow),
                                Label::new((filename.clone(), invoke.1.into_range())).with_color(
                                    if invoke.ident() == scope.ident() {
                                        Color::Blue
                                    } else {
                                        Color::Yellow
                                    },
                                ),
                            ]
                        },
                    ))
                    .with_label(
                        Label::new((filename.clone(), not_found.span().into_range()))
                            .with_color(Color::Red),
                    )
                    .with_label(
                        Label::new((filename.clone(), scope.span().into_range()))
                            .with_color(Color::Blue),
                    )
                    .with_help(format!(
                        "Ensure you've correctly entered the label (case-sensitive) or {}",
                        "make sure to define it."
                    ))
                    .finish()
            }
            Self::TableLabelNotFound {
                scope,
                table_ref,
                table_def,
                not_found,
            } => Report::build(ReportKind::Error, filename.clone(), not_found.1.start)
                .with_config(Config::default().with_index_type(IndexType::Byte))
                .with_message(format!(
                    "Jump table {} referenced label '{}' not found in direct parent macro {}",
                    table_ref.ident().fg(Color::Magenta),
                    not_found.ident().fg(Color::Red),
                    scope.ident().fg(Color::Yellow)
                ))
                .with_label(
                    Label::new((filename.clone(), scope.span().into_range()))
                        .with_color(Color::Yellow),
                )
                .with_label(
                    Label::new((filename.clone(), table_ref.span().into_range()))
                        .with_color(Color::Magenta),
                )
                .with_label(
                    Label::new((filename.clone(), table_def.span().into_range()))
                        .with_color(Color::Magenta),
                )
                .with_label(
                    Label::new((filename.clone(), not_found.span().into_range()))
                        .with_color(Color::Red),
                )
                .with_help(concat!(
                    "Unlike macro label references, jump tables can only reference labels in their",
                    " direct parent macro"
                ))
                .finish(),
            Self::MacroArgumentCountMismatch {
                scope: _,
                invoke,
                args,
                target,
            } => {
                let has_s = if target.args.0.len() == 1 { "" } else { "s" };
                let invoke_arg_span = args.1.start + 1..args.1.end - 1;

                Report::build(ReportKind::Error, filename.clone(), target.span().start)
                    .with_config(Config::default().with_index_type(IndexType::Byte))
                    .with_message(format!(
                        "Macro '{}' takes {} argument{}, invoked with {}",
                        target.ident().fg(Color::Blue),
                        target.args.0.len(),
                        has_s,
                        args.0.len()
                    ))
                    .with_label(
                        Label::new((filename.clone(), invoke.span().into_range()))
                            .with_color(Color::Blue),
                    )
                    .with_label(
                        Label::new((filename.clone(), invoke_arg_span))
                            .with_color(Color::Red)
                            .with_message(format!(
                                "Input argument count ({}) != expected count ({})",
                                args.0.len(),
                                target.args.0.len(),
                            )),
                    )
                    .with_help(
                        "Add/Remove the invalid arguments or change the macro being invoked.",
                    )
                    .finish()
            }
            Self::EntryPointHasArgs { target } => {
                let inner_arg_span = target.args.1.start + 1..target.args.1.end - 1;

                Report::build(ReportKind::Error, filename.clone(), target.span().start)
                    .with_config(Config::default().with_index_type(IndexType::Byte))
                    .with_message(format!(
                        "Entry point macro '{}' is expected to have 0 arguments, found {}",
                        target.ident().fg(Color::Blue),
                        target.args.0.len()
                    ))
                    .with_label(
                        Label::new((filename.clone(), target.span().into_range()))
                            .with_color(Color::Blue),
                    )
                    .with_label(
                        Label::new((filename.clone(), inner_arg_span))
                            .with_color(Color::Red)
                            .with_message("Should be empty"),
                    )
                    .with_help(format!(
                        "Remove the arguments from the entry point. If you need {}{}",
                        "a customizable top-level contract use constant-overrides with -c",
                        " or rename the macro and instantiate it from the entrypoint."
                    ))
                    .finish()
            }
            Self::DuplicateMacroArgDefinition { scope, duplicates } => {
                let dups_start = duplicates.iter().map(|dup| dup.1.start).min().unwrap();
                let arg_name = duplicates.first().unwrap().0;

                Report::build(ReportKind::Error, filename.clone(), dups_start)
                    .with_config(Config::default().with_index_type(IndexType::Byte))
                    .with_message(format!(
                        "Duplicate macro argument '{}' defined in '{}'.{}",
                        arg_name.fg(Color::Red),
                        scope.ident().fg(Color::Blue),
                        " Macro arguments must have unique identifiers."
                    ))
                    .with_label(
                        Label::new((filename.clone(), scope.span().into_range()))
                            .with_color(Color::Blue),
                    )
                    .with_labels(duplicates.iter().map(|dup| {
                        Label::new((filename.clone(), dup.1.into_range())).with_color(Color::Red)
                    }))
                    .with_help("Rename the arguments such that each name is unique")
                    .finish()
            }
            Self::DuplicateLabelDefinition { scope, duplicates } => {
                let dups_start = duplicates.iter().map(|dup| dup.1.start).min().unwrap();
                let label_name = duplicates.first().unwrap().0;

                Report::build(ReportKind::Error, filename.clone(), dups_start)
                    .with_config(Config::default().with_index_type(IndexType::Byte))
                    .with_message(format!(
                        "Duplicate label '{}' defined in '{}'.{}",
                        label_name.fg(Color::Red),
                        scope.ident().fg(Color::Blue),
                        " Label definitions must be unique in every macro."
                    ))
                    .with_label(
                        Label::new((filename.clone(), scope.span().into_range()))
                            .with_color(Color::Blue),
                    )
                    .with_labels(duplicates.iter().map(|dup| {
                        Label::new((filename.clone(), dup.1.into_range())).with_color(Color::Red)
                    }))
                    .with_help("Rename the labels such that each definition is unique")
                    .finish()
            }
            Self::NotYetSupported { intent, span } => {
                Report::build(ReportKind::Error, filename.clone(), span.start)
                    .with_config(Config::default().with_index_type(IndexType::Byte))
                    .with_message(format!("{} is not yet supported", intent.fg(Color::Cyan),))
                    .with_label(
                        Label::new((filename.clone(), span.into_range())).with_color(Color::Red),
                    )
                    .finish()
            }
            Self::RecursiveCodeInclusion { linking_inclusions } => {
                let recursing_inclusion = linking_inclusions.last().unwrap().inclusion;
                let recursing_name = recursing_inclusion.ident();

                let base_report = Report::build(
                    ReportKind::Error,
                    filename.clone(),
                    recursing_inclusion.1.start,
                )
                .with_config(Config::default().with_index_type(IndexType::Byte))
                .with_message(format!(
                    "Macro {} cannot be included because it recursively includes itself",
                    recursing_name.fg(Color::Red),
                ));

                linking_inclusions
                    .iter()
                    .enumerate()
                    .skip_while(|(_i, inclusion)| inclusion.entry_point.ident() != recursing_name)
                    .fold(base_report, |report, (i, inclusion)| {
                        let report = report.with_label(
                            Label::new((filename.clone(), inclusion.entry_point.1.into_range()))
                                .with_color(Color::Blue),
                        );

                        let report = inclusion.invoke_stack.iter().fold(
                            report,
                            |report, (scope, invoking)| {
                                report
                                    .with_label(
                                        Label::new((filename.clone(), scope.1.into_range()))
                                            .with_color(Color::Red),
                                    )
                                    .with_label(
                                        Label::new((filename.clone(), invoking.1.into_range()))
                                            .with_color(Color::Yellow),
                                    )
                            },
                        );

                        let is_last = i == linking_inclusions.len() - 1;
                        if !is_last {
                            report.with_label(
                                Label::new((filename.clone(), inclusion.inclusion.1.into_range()))
                                    .with_color(Color::Yellow),
                            )
                        } else {
                            report.with_label(
                                Label::new((filename.clone(), inclusion.inclusion.1.into_range()))
                                    .with_message("Recursing inclusion")
                                    .with_color(Color::Red),
                            )
                        }
                    })
                    .with_help(
                        "__codeoffset/__codesize attempts to include the target and all \
                        its dependencies, which it cannot do if the dependencies are cyclic.",
                    )
                    .finish()
            }
        }
    }
}
