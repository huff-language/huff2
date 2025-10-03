use ariadne::{sources, Color, Config, Fmt, IndexType, Label, Report, ReportKind};
use clap::Parser as ClapParser;
use huff_analysis::{const_overrides::*, *};
use huff_ast::{parse, RootSection};
use huff_compilation::{generate_default_constructor, generate_for_entrypoint, CompileGlobals};
use std::collections::BTreeSet;

mod versions;
use versions::EvmVersion;

/// Huff Language Compiler
#[derive(ClapParser)]
struct CliArguments {
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
        help = "wraps target entry point code in a minimal constructor that deploys it"
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
        short = 'c',
        long = "constant",
        value_parser = parse_constant_override,
        help = "Add override to list in format <CONSTANT_NAME>=<HEX/DEC VALUE>"
    )]
    constant_overrides: Vec<ConstantOverride>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArguments::parse();

    let src_res = std::fs::read_to_string(&args.filename);

    if let Err(err) = &src_res {
        if let std::io::ErrorKind::NotFound = err.kind() {
            eprintln!(
                "{}: File with path '{}' not found",
                "Error".fg(Color::Red),
                args.filename.escape_debug()
            );
            std::process::exit(1);
        }
    };

    {
        let mut unique_overrids = BTreeSet::new();
        let mut found_duplicate = false;
        for const_override in &args.constant_overrides {
            if !unique_overrids.insert(const_override.name.as_str()) {
                eprintln!(
                    "{}: Duplicate override for constant {}",
                    "Error".fg(Color::Red),
                    const_override.name.as_str().fg(Color::Yellow)
                );
                found_duplicate = true;
            }
        }
        if found_duplicate {
            std::process::exit(1);
        }
    }

    let src = src_res?;

    let filename: String = args.filename;

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
    verify_constants_to_be_overriden_defined(&global_defs, &args.constant_overrides, |err| {
        analysis_errors.push(err)
    });

    {
        let mut to_analyze_stack = vec![CodeInclusionFrame::top(args.entry_point.as_str())];
        let mut analyzed_macros = BTreeSet::new();
        while let Some(next_entrypoint) = to_analyze_stack.last() {
            let idx_to_remove = to_analyze_stack.len() - 1;
            if analyzed_macros.insert(next_entrypoint.name) {
                analyze_entry_point(
                    &global_defs,
                    next_entrypoint.name,
                    |err| analysis_errors.push(err),
                    &mut to_analyze_stack,
                );
            }
            to_analyze_stack.remove(idx_to_remove);
        }
    }

    if !analysis_errors.is_empty() {
        analysis_errors.into_iter().for_each(|err| {
            err.report(filename.clone())
                .eprint(sources([(filename.clone(), &src)]))
                .unwrap()
        });
        std::process::exit(1);
    }

    let mut config = CompileGlobals::new(
        true,
        args.evm_version.allows_push0(),
        unique_defs,
        &args.constant_overrides,
    );

    let entry_point_macro = match config.defs.get(args.entry_point.as_str()) {
        Some(huff_ast::Definition::Macro(entry_point)) => entry_point,
        _ => panic!("macro not found despite no errors in analysis"),
    };
    let mut entry_point_code = generate_for_entrypoint(&mut config, entry_point_macro);
    if args.add_default_constructor {
        entry_point_code = config.assemble(&generate_default_constructor(entry_point_code));
    }

    println!("0x{}", hex::encode(entry_point_code));

    Ok(())
}
