use ariadne::{sources, Color, Config, IndexType, Label, Report, ReportKind};
use clap::Parser as ClapParser;
use huff_analysis::*;
use huff_ast::{parse, RootSection};
use thiserror::Error;

/// Huff Language Compiler
#[derive(ClapParser)]
struct Cli {
    /// filename
    #[clap(help = "Root huff file to compile")]
    filename: String,

    #[clap(
        short = 'm',
        long = "alt-main",
        default_value = "MAIN",
        help = "alternative entry point for runtime execution"
    )]
    main: String,
    #[clap(
        short = 't',
        long = "alt-constructor",
        default_value = "CONSTRUCTOR",
        help = "entry point for constructor (initcode) execution"
    )]
    constructor: String,
}

#[derive(Error, Debug)]
enum HuffError {
    /// Wrapper around `io::Error`
    #[error("{0}")]
    Io(#[from] std::io::Error),
    // #[error("{0}")]
    // Parser(Report),
    // Parser(#[from] ParseError<usize, Token<'src>, huff_ast::Error>),
}
type HuffResult = Result<(), HuffError>;

fn main() -> HuffResult {
    let cli = Cli::parse();

    let src = std::fs::read_to_string(&cli.filename)?;
    let filename: String = cli.filename;

    let ast = match parse(&src) {
        Ok(ast) => ast,
        Err(errs) => {
            errs.into_iter().for_each(|e| {
                Report::build(ReportKind::Error, filename.clone(), e.span().start)
                    .with_config(Config::default().with_index_type(IndexType::Byte))
                    // .with_message(e.reason())
                    .with_label(
                        Label::new((filename.clone(), e.span().into_range()))
                            .with_message(e.reason())
                            .with_color(Color::Red),
                    )
                    .finish()
                    .print(sources([(filename.clone(), &src)]))
                    .unwrap()
            });

            std::process::exit(1);
        }
    };

    let mut analysis_errors = Vec::with_capacity(5);
    let global_defs = build_ident_map(ast.0.iter().map(|section| match section {
        RootSection::Include(_) => todo!("Include not yet supported"),
        RootSection::Definition(def) => def,
    }));
    let _unique_defs = analyze_global_for_dups(&global_defs, |err| analysis_errors.push(err));

    Ok(())
}
