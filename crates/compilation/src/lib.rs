use alloy_primitives::U256;
use evm_glue::{assemble_maximized, assemble_minimized, utils::MarkTracker};
use evm_glue::{
    assembly::{Asm, MarkRef, RefType},
    opcodes::Opcode,
};
use huff_analysis::label_stack::LabelStack;
use huff_ast::*;
use std::collections::{BTreeMap, HashMap};

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

struct IncludedObject {
    start_id: usize,
    end_id: usize,
    body: Box<[Asm]>,
}

pub fn generate_for_entrypoint<'src>(
    globals: &mut CompileGlobals<'src, '_>,
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

    let mut objects: Vec<IncludedObject> = Vec::with_capacity(10);

    let mut asm = Vec::with_capacity(10_000);
    asm.push(Asm::Mark(start_id));
    generate_for_macro(
        globals,
        entry_point,
        Box::new([]),
        &mut mark_tracker,
        &mut label_stack,
        &mut included_macros,
        &mut objects,
        &mut asm,
    );

    included_macros.into_iter().skip(1).for_each(|included| {
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

    objects.into_iter().for_each(|obj| {
        asm.push(Asm::Mark(obj.start_id));
        asm.extend(obj.body);
        asm.push(Asm::Mark(obj.end_id));
    });

    asm.push(Asm::Mark(end_id));

    globals.assemble(asm.as_slice())
}

pub fn generate_default_constructor(runtime: Vec<u8>) -> Box<[Asm]> {
    let mut mtracker = MarkTracker::default();
    let runtime_start = mtracker.next_mark();
    let runtime_end = mtracker.next_mark();
    Box::new([
        // Constructor
        Asm::delta_ref(runtime_start, runtime_end), // rt_size
        Asm::Op(Opcode::DUP1),                      // rt_size, rt_size
        Asm::mref(runtime_start),                   // rt_size, rt_size, rt_start
        Asm::Op(Opcode::RETURNDATASIZE),            // rt_size, rt_size, rt_start, 0
        Asm::Op(Opcode::CODECOPY),                  // rt_size
        Asm::Op(Opcode::RETURNDATASIZE),            // rt_size, 0
        Asm::Op(Opcode::RETURN),                    // -- end
        // Runtime body
        Asm::Mark(runtime_start),
        Asm::Data(runtime),
        Asm::Mark(runtime_end),
    ])
}

struct IncludedJumpTable {
    start_id: usize,
    end_id: usize,
}

#[allow(clippy::too_many_arguments)]
fn generate_for_macro<'src: 'cmp, 'cmp>(
    globals: &mut CompileGlobals<'src, '_>,
    current: &Macro<'src>,
    arg_values: Box<[Asm]>,
    mark_tracker: &mut MarkTracker,
    label_stack: &'cmp mut LabelStack<'src, usize>,
    included_macros: &'cmp mut Vec<IncludedMacro<'src>>,
    objects: &'cmp mut Vec<IncludedObject>,
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

    let mut jump_tables: HashMap<&str, IncludedJumpTable> = HashMap::with_capacity(2);

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
                    included_macros,
                    objects,
                    asm,
                )
            }
            Invoke::BuiltinCodeSize(code_ref) => {
                let mref: MarkRef = if let Some(included) =
                    included_macros.iter().find(|m| m.name == code_ref.ident())
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
                    included_macros.push(included);
                    mref
                };
                asm.push(Asm::Ref(mref));
            }
            Invoke::BuiltinCodeOffset(code_ref) => {
                let mref: MarkRef = if let Some(included) =
                    included_macros.iter().find(|m| m.name == code_ref.ident())
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
                    included_macros.push(included);
                    mref
                };
                asm.push(Asm::Ref(mref));
            }
            Invoke::BuiltinTableSize(table_ref) => {
                let jump_table = match globals.defs.get(table_ref.ident()).unwrap() {
                    Definition::Jumptable(jump_table) => jump_table,
                    other_def => panic!("Unexpected table def {:?}", other_def),
                };
                let included_table = jump_tables.entry(table_ref.ident()).or_insert_with(|| {
                    let start_id = mark_tracker.next_mark();
                    let end_id = mark_tracker.next_mark();
                    objects.push(IncludedObject {
                        start_id,
                        end_id,
                        body: Box::from_iter(jump_table.0.labels.iter().map(|label| {
                            let label_id = label_stack.get(label.ident()).unwrap();
                            let mref = MarkRef {
                                ref_type: RefType::Direct(*label_id),
                                is_pushed: false,
                                set_size: Some(jump_table.0.label_size),
                            };
                            Asm::Ref(mref)
                        })),
                    });
                    IncludedJumpTable { start_id, end_id }
                });
                asm.push(Asm::delta_ref(
                    included_table.start_id,
                    included_table.end_id,
                ));
            }
            Invoke::BuiltinTableStart(table_ref) => {
                let jump_table = match globals.defs.get(table_ref.ident()).unwrap() {
                    Definition::Jumptable(jump_table) => jump_table,
                    other_def => panic!("Unexpected table def {:?}", other_def),
                };
                let included_table = jump_tables.entry(table_ref.ident()).or_insert_with(|| {
                    let start_id = mark_tracker.next_mark();
                    let end_id = mark_tracker.next_mark();
                    objects.push(IncludedObject {
                        start_id,
                        end_id,
                        body: Box::from_iter(jump_table.0.labels.iter().map(|label| {
                            let label_id = label_stack.get(label.ident()).unwrap();
                            let mref = MarkRef {
                                ref_type: RefType::Direct(*label_id),
                                is_pushed: false,
                                set_size: Some(jump_table.0.label_size),
                            };
                            Asm::Ref(mref)
                        })),
                    });
                    IncludedJumpTable { start_id, end_id }
                });
                asm.push(Asm::mref(included_table.start_id));
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
