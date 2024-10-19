use argh::FromArgs;
use ariadne::{sources, Color, Config, IndexType, Label, Report, ReportKind};
use huff_ast::parse;
use std::{fs::read_to_string, io, process::exit};
use thiserror::Error;

fn main() {
    let cli: Cli = argh::from_env();
    let res = match cli.command {
        Commands::Build(cmd) => build(cmd),
    };
    if let Err(e) = res {
        eprintln!("error: {}", e);
        exit(1);
    }
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

fn build(cmd: BuildCommand) -> HuffResult {
    let src = read_to_string(&cmd.filename)?;
    let filename: String = cmd.filename;
    match parse(&src) {
        Ok(ast) => println!("{:?}", ast),
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
        }
    }

    Ok(())
}

#[derive(FromArgs)]
/// Huff Language Compiler
struct Cli {
    #[argh(subcommand)]
    command: Commands,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Commands {
    Build(BuildCommand),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "build")]
/// Build compiles a huff file.
struct BuildCommand {
    /// filename
    #[argh(positional)]
    filename: String,
}
