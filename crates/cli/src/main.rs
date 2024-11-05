use ariadne::{sources, Color, Config, Fmt, IndexType, Label, Report, ReportKind};
use clap::Parser as ClapParser;
use evm_glue::{assemble_maximized, assemble_minimized, utils::MarkTracker};
use huff_analysis::*;
use huff_ast::{parse, RootSection};
use huff_compilation::{generate_default_constructor, generate_for_entrypoint, CompileGlobals};

mod versions;
use versions::EvmVersion;

/// Huff Language Compiler
#[derive(ClapParser)]
struct Cli {
    /// filename
    #[clap(help = "Root huff file to compile")]
    filename: String,

    #[clap(
        help = "Name of Huff entrypoint macro to compile to EVM bytecode. NOTE: Will compile the entry point *as is*, no implicit initcode wrapper."
    )]
    entry_point: String,

    #[clap(
        short = 'f',
        long = "default-constructor",
        help = "whether to wrap target entry point code in a minimal constructor"
    )]
    add_default_constructor: bool,

    #[clap(
        short = 'e',
        long = "evm-version",
        help = "What EVM version to use, NOTE: Pre-EOF this option only affects the use of PUSH0.",
        default_value = "paris"
    )]
    evm_version: EvmVersion,

    #[clap(
        short = 'z',
        long = "optimize",
        help = "Whether to optimize the resulting assembly. NOTE: Currently only toggles minimization of push opcodes for label references",
        default_value_t = true
    )]
    optimize: bool,
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

    let entry_point_macro = analyze_entry_point(&global_defs, cli.entry_point.as_str(), |err| {
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
    let mut config = CompileGlobals::new(cli.evm_version.allows_push0(), unique_defs);

    let asm = match generate_for_entrypoint(&mut config, entry_point_macro.unwrap(), &mut mtracker)
    {
        Ok(asm) => asm,
        Err(reason) => {
            eprintln!("{}: {}", "Error".fg(Color::Red), reason);
            std::process::exit(1);
        }
    };

    let assemble = if cli.optimize {
        assemble_minimized
    } else {
        assemble_maximized
    };

    let (_, mut entry_point_code) = assemble(asm.as_slice(), config.allow_push0).unwrap();

    if cli.add_default_constructor {
        (_, entry_point_code) = assemble(
            &generate_default_constructor(entry_point_code),
            config.allow_push0,
        )
        .unwrap();
    }

    println!("0x{}", hex::encode(entry_point_code));

    Ok(())
}
