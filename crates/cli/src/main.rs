use clap::Parser as ClapParser;
use huff_ast::parse;
use std::{fs::read_to_string, io, process::exit};
use thiserror::Error;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {}", e);
        exit(1);
    }
}

fn run(cli: Cli) -> HuffResult {
    let src = read_to_string(&cli.filename)?;
    match parse(&src) {
        Ok(ast) => println!("{:?}", ast),
        Err(e) => println!("error: {:?}", e),
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

#[derive(ClapParser)]
#[command(name = "huff")]
struct Cli {
    /// Filename
    filename: String,
}
