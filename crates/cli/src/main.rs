use argh::FromArgs;
use ariadne::{sources, Color, Label, Report, ReportKind, Source};
use huff_ast::parse;
use std::{fs::read_to_string, io, process::exit};
use thiserror::Error;

fn main() {
    let cli = argh::from_env();
    if let Err(e) = run(cli) {
        eprintln!("error: {}", e);
        exit(1);
    }
}

fn run(cli: Cli) -> HuffResult {
    let src = read_to_string(&cli.filename)?;
    let filename: String = cli.filename;
    match parse(&src) {
        Ok(ast) => println!("{:?}", ast),
        Err(errs) => errs.into_iter().for_each(|e| {
            Report::build(ReportKind::Error, filename.clone(), e.span().start)
                .with_message(e.reason())
                .with_label(
                    Label::new((filename.clone(), e.span().into_range()))
                        .with_message(e.reason())
                        .with_color(Color::Red),
                )
                .finish()
                .print(sources([(filename.clone(), &src)]))
                .unwrap()
        }),
    }

    Ok(())
}

#[derive(Error, Debug)]
enum HuffError {
    /// Wrapper around `io::Error`
    #[error("{0}")]
    Io(#[from] io::Error),
    // #[error("{0}")]
    // Parser(Report),
    // Parser(#[from] ParseError<usize, Token<'src>, huff_ast::Error>),
}

type HuffResult = Result<(), HuffError>;

#[derive(FromArgs)]
/// Huff Language Compiler
struct Cli {
    /// filename
    #[argh(positional)]
    filename: String,
}
