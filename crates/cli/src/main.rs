use ariadne::{sources, Color, Config, IndexType, Label, Report, ReportKind};
use clap::Parser as ClapParser;
use evm_glue::{assemble_minimized, utils::MarkTracker};
use huff_analysis::*;
use huff_ast::{parse, Definition, RootSection};
use huff_compilation::{generate_for_entrypoint, CompileConfig};
use thiserror::Error;

mod versions;
use versions::EvmVersion;

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
    #[clap(
        short = 'b',
        long = "bytecode",
        help = "Generate and log constructor bytecode (aka initcode)"
    )]
    initcode: bool,
    #[clap(
        short = 'r',
        long = "runtime",
        help = "Generate and log runtime bytecode"
    )]
    runtime: bool,
    #[clap(
        short = 'e',
        long = "evm-version",
        help = "What EVM version to use, NOTE: Pre-EOF this option only affects the use of PUSH0.",
        default_value = "paris"
    )]
    evm_version: EvmVersion,
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
    let unique_defs = analyze_global_for_dups(&global_defs, |err| analysis_errors.push(err));
    let main = unique_defs
        .get(cli.main.as_str())
        .and_then(|def| match def {
            Definition::Macro(m) => Some(m),
            _ => None,
        })
        .unwrap_or_else(|| {
            todo!(
                "Runtime entrypoint {} not found, not unique or not macro",
                cli.main
            )
        });
    analyze_entry_point(&global_defs, main, |err| analysis_errors.push(err));

    if !analysis_errors.is_empty() {
        analysis_errors.into_iter().for_each(|err| {
            eprintln!("{:?}", err);
        });
        std::process::exit(1);
    }

    let mut mtracker = MarkTracker::default();
    let config = CompileConfig {
        allow_push0: cli.evm_version.allows_push0(),
    };
    let asm = generate_for_entrypoint(&unique_defs, main, &mut mtracker, &config);

    let code = assemble_minimized(asm.as_slice(), config.allow_push0).unwrap();

    println!("runtime: 0x{}", hex::encode(code));

    Ok(())
}
