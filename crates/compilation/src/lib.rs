use alloy_primitives::U256;
use evm_glue::{assemble_maximized, assemble_minimized, evm_asm, utils::MarkTracker};
use evm_glue::{
    assembly::{Asm, MarkRef, RefType},
    opcodes::Opcode,
};
use huff_analysis::label_stack::LabelStack;
use huff_ast::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct IncludedMacro<'src> {
    name: &'src str,
    start_id: usize,
    end_id: usize,
}

impl IncludedMacro<'_> {
    fn size_ref(&self) -> MarkRef {
        MarkRef {
            ref_type: RefType::Delta(self.start_id, self.end_id),
            is_pushed: true,
            set_size: None,
        }
    }

    fn start_ref(&self) -> MarkRef {
        MarkRef {
            ref_type: RefType::Direct(self.start_id),

            is_pushed: true,
            set_size: None,
        }
    }
}

struct IncludedCodeTable<'src, 'ast> {
    name: &'src str,
    referenced: bool,
    start_id: usize,
    end_id: usize,
    data: &'ast [u8],
}

struct ProgramDataDeps<'src, 'ast> {
    included_macros: Vec<IncludedMacro<'src>>,
    included_code_tables: Vec<IncludedCodeTable<'src, 'ast>>,
}

impl<'src, 'ast> IncludedCodeTable<'src, 'ast> {
    fn size_ref(&self) -> MarkRef {
        MarkRef {
            ref_type: RefType::Delta(self.start_id, self.end_id),
            is_pushed: true,
            set_size: None,
        }
    }

    fn start_ref(&self) -> MarkRef {
        MarkRef {
            ref_type: RefType::Direct(self.start_id),
            is_pushed: true,
            set_size: None,
        }
    }
}

pub fn generate_for_entrypoint<'src, 'ast: 'src>(
    globals: &mut CompileGlobals<'src, 'ast>,
    entry_point: &Macro<'src>,
) -> Vec<u8> {
    let mut mark_tracker = MarkTracker::default();
    let mut label_stack: LabelStack<usize> = LabelStack::default();

    let mut included_macros: Vec<IncludedMacro> = Vec::with_capacity(4);
    let start_id = mark_tracker.next_mark();
    let end_id = mark_tracker.next_mark();
    included_macros.push(IncludedMacro {
        name: entry_point.ident(),
        start_id,
        end_id,
    });
    let included_code_tables: Vec<IncludedCodeTable<'src, 'ast>> = globals
        .defs
        .iter()
        .filter_map(|(_, def)| {
            let Definition::CodeTable { name, data } = def else {
                return None;
            };
            Some(IncludedCodeTable {
                name: name.ident(),
                referenced: false,
                data,
                start_id: mark_tracker.next_mark(),
                end_id: mark_tracker.next_mark(),
            })
        })
        .collect();

    let mut program_data_deps = ProgramDataDeps {
        included_macros,
        included_code_tables,
    };

    let mut asm = Vec::with_capacity(10_000);
    asm.push(Asm::Mark(start_id));
    generate_for_macro(
        globals,
        entry_point,
        Box::new([]),
        &mut mark_tracker,
        &mut label_stack,
        &mut program_data_deps,
        &mut asm,
    );

    program_data_deps
        .included_macros
        .into_iter()
        .skip(1)
        .for_each(|included| {
            let section_macro =
                if let Some(Definition::Macro(section_macro)) = globals.defs.get(included.name) {
                    section_macro
                } else {
                    panic!("Section macro {} not found", included.name);
                };
            asm.push(Asm::Mark(included.start_id));
            asm.push(Asm::Data(generate_for_entrypoint(globals, section_macro)));
            asm.push(Asm::Mark(included.end_id));
        });

    program_data_deps
        .included_code_tables
        .into_iter()
        .filter(|t| t.referenced)
        .for_each(|included| {
            asm.push(Asm::Mark(included.start_id));
            asm.push(Asm::Data(included.data.to_vec()));
            asm.push(Asm::Mark(included.end_id));
        });

    asm.push(Asm::Mark(end_id));

    globals.assemble(asm.as_slice())
}

/// WARNING: Only to be used as standalone constructor, may break if added after other code due to
/// reliance on `RETURNDATASIZE` being `0`.
pub fn generate_default_constructor(runtime: Vec<u8>) -> Box<[Asm]> {
    use Opcode::*;

    match runtime.len() {
        0 => Vec::new(),
        1..=32 => {
            let code_push = u256_as_push(U256::from_be_slice(runtime.as_slice()));
            let len: u8 = runtime.len().try_into().unwrap();

            if len == 32 {
                evm_asm!(
                    Op(code_push),
                    RETURNDATASIZE,
                    MSTORE,
                    MSIZE,
                    RETURNDATASIZE,
                    RETURN
                )
            } else {
                evm_asm!(
                    Op(code_push),
                    RETURNDATASIZE,
                    MSTORE,
                    PUSH1([len]),
                    PUSH1([32 - len]),
                    RETURN
                )
            }
        }
        _ => {
            let mut mtracker = MarkTracker::default();
            let runtime_start = mtracker.next_mark();
            let runtime_end = mtracker.next_mark();
            evm_asm!(
                // Constructor
                Asm::delta_ref(runtime_start, runtime_end), // rt_size
                DUP1,                                       // rt_size, rt_size
                Asm::mref(runtime_start),                   // rt_size, rt_size, rt_start
                RETURNDATASIZE,                             // rt_size, rt_size, rt_start, 0
                CODECOPY,                                   // rt_size
                RETURNDATASIZE,                             // rt_size, 0
                RETURN,                                     // -- end
                // Runtime body
                Mark(runtime_start),
                Data(runtime),
                Mark(runtime_end),
            )
        }
    }
    .into_boxed_slice()
}

fn generate_for_macro<'src: 'cmp, 'cmp, 'ast>(
    globals: &mut CompileGlobals<'src, 'ast>,
    current: &Macro<'src>,
    arg_values: Box<[Asm]>,
    mark_tracker: &mut MarkTracker,
    label_stack: &'cmp mut LabelStack<'src, usize>,
    program_data_deps: &'cmp mut ProgramDataDeps<'src, 'ast>,
    asm: &mut Vec<Asm>,
) {
    let current_args: BTreeMap<&str, Asm> = BTreeMap::from_iter(
        current
            .args
            .0
            .iter()
            .map(|name| name.ident())
            .zip(arg_values),
    );

    label_stack.enter_context();

    current.body.iter().for_each(|stmt| {
        if let MacroStatement::LabelDefinition(name) = stmt {
            label_stack.push(name.ident(), mark_tracker.next_mark());
        }
    });

    current.body.iter().for_each(|stmt| match stmt {
        MacroStatement::LabelDefinition(name) => {
            asm.extend([
                Asm::Mark(*label_stack.get(name.ident()).unwrap()),
                Asm::Op(Opcode::JUMPDEST),
            ]);
        }
        MacroStatement::Invoke(invoke) => match invoke {
            Invoke::Macro { name, args } => {
                let target =
                    if let Definition::Macro(target) = globals.defs.get(name.ident()).unwrap() {
                        target
                    } else {
                        panic!("Target should've been validated to be macro")
                    };
                generate_for_macro(
                    globals,
                    target,
                    args.0
                        .iter()
                        .map(|arg| instruction_to_asm(globals, &current_args, label_stack, arg))
                        .collect(),
                    mark_tracker,
                    label_stack,
                    program_data_deps,
                    asm,
                )
            }
            Invoke::BuiltinCodeSize(code_ref) => {
                let mref: MarkRef = if let Some(included) = program_data_deps
                    .included_macros
                    .iter()
                    .find(|m| m.name == code_ref.ident())
                {
                    included.size_ref()
                } else {
                    let start_id = mark_tracker.next_mark();
                    let end_id = mark_tracker.next_mark();
                    let included = IncludedMacro {
                        name: code_ref.ident(),
                        start_id,
                        end_id,
                    };
                    let mref = included.size_ref();
                    program_data_deps.included_macros.push(included);
                    mref
                };
                asm.push(Asm::Ref(mref));
            }
            Invoke::BuiltinCodeOffset(code_ref) => {
                let mref: MarkRef = if let Some(included) = program_data_deps
                    .included_macros
                    .iter()
                    .find(|m| m.name == code_ref.ident())
                {
                    included.start_ref()
                } else {
                    let start_id = mark_tracker.next_mark();
                    let end_id = mark_tracker.next_mark();
                    let included = IncludedMacro {
                        name: code_ref.ident(),
                        start_id,
                        end_id,
                    };
                    let mref = included.start_ref();
                    program_data_deps.included_macros.push(included);
                    mref
                };
                asm.push(Asm::Ref(mref));
            }
            Invoke::BuiltinTableStart(table_ref) => {
                let target_table = program_data_deps
                    .included_code_tables
                    .iter_mut()
                    .find(|t| t.name == table_ref.ident())
                    .expect("Table not found (might be jumptable)");
                target_table.referenced = true;
                asm.push(Asm::Ref(target_table.start_ref()));
            }
            Invoke::BuiltinTableSize(table_ref) => {
                let target_table = program_data_deps
                    .included_code_tables
                    .iter_mut()
                    .find(|t| t.name == table_ref.ident())
                    .expect("Table not found (might be jumptable)");
                target_table.referenced = true;
                asm.push(Asm::Ref(target_table.size_ref()));
            }
            Invoke::BuiltinFuncSig(func) => {
                let Definition::SolFunction(sol_func) = globals.defs[func.ident()] else {
                    unreachable!(
                        "Reached codegen even though \"{}\" not found in global defs",
                        func.ident()
                    )
                };
                let selector = compute_selector(&sol_func.name, &sol_func.args);
                asm.push(u256_to_asm(
                    U256::from_be_slice(selector.as_slice()),
                    globals.allow_push0,
                ));
            }
            _ => panic!(
                "Compilation not yet implemented for this invocation type `{:?}`",
                invoke
            ),
        },
        MacroStatement::Instruction(ref i) => {
            asm.push(instruction_to_asm(globals, &current_args, label_stack, i));
        }
    });

    label_stack.leave_context();
}

fn instruction_to_asm(
    globals: &CompileGlobals,
    args: &BTreeMap<&str, Asm>,
    label_stack: &LabelStack<usize>,
    i: &Instruction,
) -> Asm {
    match i {
        Instruction::Op((op, _)) => Asm::Op(*op),
        Instruction::VariablePush((value, _)) => u256_to_asm(*value, globals.allow_push0),
        Instruction::LabelReference(name) => Asm::mref(*label_stack.get(name.ident()).unwrap()),
        Instruction::ConstantReference(name) => u256_to_asm(
            *globals.constants.get(name.ident()).unwrap(),
            globals.allow_push0,
        ),
        Instruction::MacroArgReference(name) => args.get(name.ident()).unwrap().clone(),
    }
}

pub fn u256_to_asm(value: U256, allow_push0: bool) -> Asm {
    Asm::Op(if value.byte_len() == 0 && allow_push0 {
        Opcode::PUSH0
    } else {
        u256_as_push(value)
    })
}

#[derive(Debug, Clone)]
pub struct CompileGlobals<'src, 'ast> {
    pub minimize: bool,
    pub allow_push0: bool,
    pub defs: BTreeMap<&'src str, &'ast Definition<'src>>,
    pub constants: BTreeMap<&'src str, U256>,
}

impl<'src, 'ast> CompileGlobals<'src, 'ast> {
    pub fn new(
        minimize: bool,
        allow_push0: bool,
        defs: BTreeMap<&'src str, &'ast Definition<'src>>,
    ) -> Self {
        let constants = evalute_constants(&defs);
        Self {
            minimize,
            allow_push0,
            defs,
            constants,
        }
    }

    pub fn assemble(&self, asm: &[Asm]) -> Vec<u8> {
        if self.minimize {
            assemble_minimized(asm, self.allow_push0)
        } else {
            assemble_maximized(asm, self.allow_push0)
        }
        .unwrap()
        .1
    }
}

fn evalute_constants<'a>(global_defs: &BTreeMap<&'a str, &Definition>) -> BTreeMap<&'a str, U256> {
    let mut free_pointer = 0u32;
    global_defs
        .iter()
        .filter_map(|(name, def)| match def {
            Definition::Constant { name: _, expr } => Some((name, expr)),
            _ => None,
        })
        .map(|(name, expr)| match expr.0 {
            ConstExpr::Value(v) => (*name, v),
            ConstExpr::FreeStoragePointer => {
                let current = free_pointer;
                free_pointer += 1;
                (*name, U256::from(current))
            }
        })
        .collect()
}
