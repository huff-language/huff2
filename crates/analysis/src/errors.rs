use ariadne::{Color, Config, Fmt, IndexType, Label, Report, ReportKind};
use huff_ast::{Definition, IdentifiableNode, Instruction, Macro, Spanned};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisError<'ast, 'src> {
    /// When two different definitions have the same.
    DefinitionNameCollision {
        collided: Box<[&'ast Definition<'src>]>,
        duplicate_name: &'src str,
    },
    RecursiveMacroInvocation {
        invocation_chain: Box<[(&'ast Macro<'src>, &'ast Spanned<&'src str>)]>,
    },
    LabelNotFound {
        scope: &'ast Macro<'src>,
        invocation_chain: Box<[(&'ast Macro<'src>, &'ast Spanned<&'src str>)]>,
        not_found: &'ast Spanned<&'src str>,
    },
    MacroArgNotFound {
        scope: &'ast Macro<'src>,
        not_found: &'ast Spanned<&'src str>,
    },
    EntryPointNotFound {
        name: &'src str,
    },
    DefinitionNotFound {
        scope: &'ast Macro<'src>,
        def_type: &'static str,
        name: &'ast Spanned<&'src str>,
    },
    MacroArgumentCountMismatch {
        scope: Option<&'ast Macro<'src>>,
        args: &'ast [Instruction<'src>],
        target: &'ast Macro<'src>,
    },
    DuplicateLabelDefinition {
        scope: &'ast Macro<'src>,
        duplicates: Box<[&'ast Spanned<&'src str>]>,
    },
    DuplicateMacroArgDefinition {
        scope: &'ast Macro<'src>,
        duplicates: Box<[&'ast Spanned<&'src str>]>,
    },
    NotYetSupported {
        intent: String,
        span: Spanned<()>,
    },
}

impl AnalysisError<'_, '_> {
    pub fn report(&self, filename: String) -> Report<(String, std::ops::Range<usize>)> {
        match self {
            AnalysisError::DefinitionNameCollision {
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
                            duplicate_name.escape_debug().fg(Color::Red)
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
            AnalysisError::EntryPointNotFound { name } => {
                Report::build(ReportKind::Error, filename.clone(), 0)
                    .with_message(format!("Entry point '{}' not found", name.fg(Color::Red)))
                    .finish()
            }
            AnalysisError::RecursiveMacroInvocation { invocation_chain } => {
                let first_invoke = invocation_chain.first().unwrap();

                let base_report =
                    Report::build(ReportKind::Error, filename.clone(), first_invoke.1 .1.start)
                        .with_config(Config::default().with_index_type(IndexType::Byte))
                        .with_message(format!(
                            "Cannot expand macro {} with recursive dependency on itself",
                            first_invoke.0.ident().escape_debug().fg(Color::Red)
                        ));

                invocation_chain
                    .iter()
                    .enumerate()
                    .map(|(i, (scope, invoking))| {
                        (i == invocation_chain.len() - 1, scope, invoking)
                    })
                    .fold(base_report, |report, (is_last, scope, invoking)| {
                        let report = report.with_label(
                            Label::new((filename.clone(), scope.name.1.into_range()))
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
            AnalysisError::MacroArgNotFound { scope, not_found } => {
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
                    .with_label(
                        Label::new((filename.clone(), scope.args.1.into_range()))
                            .with_color(Color::Red)
                            .with_message(format!(
                                "no '{}' in argument list",
                                not_found.0.fg(Color::Red)
                            )),
                    )
                    .finish()
            }
            _ => Report::build(ReportKind::Error, filename.clone(), 0)
                .with_message(format!("Error with unimplemented formatting: {:?}", self))
                .finish(),
        }
    }
}
