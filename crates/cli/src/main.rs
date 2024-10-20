use ariadne::{sources, Color, Config, Fmt, IndexType, Label, Report, ReportKind};
use clap::Parser as ClapParser;
use evm_glue::{assemble_minimized, utils::MarkTracker};
use huff_analysis::*;
use huff_ast::{parse, RootSection};
use huff_compilation::{evalute_constants, generate_for_entrypoint, CompileGlobals};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if matches!(cli.evm_version, EvmVersion::Eof) {
        eprintln!(
            "{}: EVM Version 'EOF' not yet supported",
            "Error".fg(Color::Red),
        );
        std::process::exit(1);
    }

    let src_res = std::fs::read_to_string(&cli.filename);

    if let Err(err) = &src_res {
        if let std::io::ErrorKind::NotFound = err.kind() {
            eprintln!(
                "{}: File with path '{}' not found",
                "Error".fg(Color::Red),
                cli.filename.escape_debug()
            );
            std::process::exit(1);
        }
    };

    let src = src_res?;

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
                    .eprint(sources([(filename.clone(), &src)]))
                    .unwrap()
            });

            std::process::exit(1);
        }
    };

    let mut analysis_errors = Vec::with_capacity(5);
    let global_defs = build_ident_map(ast.0.iter().filter_map(|section| match section {
        RootSection::Include(huff_include) => {
            analysis_errors.push(errors::AnalysisError::NotYetSupported {
                intent: "Huff '#include'".to_owned(),
                span: ((), huff_include.1),
            });
            None
        }
        RootSection::Definition(def) => Some(def),
    }));
    let unique_defs = analyze_global_for_dups(&global_defs, |err| analysis_errors.push(err));

    let main_macro = analyze_entry_point(&global_defs, cli.main.as_str(), |err| {
        analysis_errors.push(err)
    });

    if !analysis_errors.is_empty() {
        analysis_errors.into_iter().for_each(|err| {
            err.report(filename.clone())
                .eprint(sources([(filename.clone(), &src)]))
                .unwrap()
        });
        std::process::exit(1);
    }

    let mut mtracker = MarkTracker::default();
    let config = {
        let constants = evalute_constants(&unique_defs);
        CompileGlobals {
            allow_push0: cli.evm_version.allows_push0(),
            defs: unique_defs,
            constants,
        }
    };
    let asm = match generate_for_entrypoint(&config, main_macro.unwrap(), &mut mtracker) {
        Ok(asm) => asm,
        Err(reason) => {
            eprintln!("{}: {}", "Error".fg(Color::Red), reason);
            std::process::exit(1);
        }
    };

    let code = assemble_minimized(asm.as_slice(), config.allow_push0).unwrap();

    println!("runtime: 0x{}", hex::encode(code));

    Ok(())
}
