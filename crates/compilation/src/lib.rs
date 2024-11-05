use alloy_primitives::U256;
use evm_glue::{assembly::Asm, opcodes::Opcode, utils::MarkTracker};
use huff_analysis::label_stack::LabelStack;
use huff_ast::*;
use std::collections::BTreeMap;

pub fn generate_for_entrypoint<'src>(
    globals: &mut CompileGlobals<'src, '_>,
    entry_point: &Macro<'src>,
    mark_tracker: &mut MarkTracker,
) -> Result<Vec<Asm>, String> {
    let mut label_stack: LabelStack<usize> = LabelStack::default();
    let mut asm = Vec::with_capacity(10_000);

    generate_for_macro(
        globals,
        entry_point,
        Box::new([]),
        mark_tracker,
        &mut label_stack,
        &mut asm,
    )?;

    Ok(asm)
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

fn generate_for_macro<'src: 'cmp, 'cmp>(
    globals: &mut CompileGlobals<'src, '_>,
    current: &Macro<'src>,
    arg_values: Box<[Asm]>,
    mark_tracker: &mut MarkTracker,
    label_stack: &'cmp mut LabelStack<'src, usize>,
    asm: &mut Vec<Asm>,
) -> Result<(), String> {
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

    let res = current.body.iter().try_for_each(|stmt| match stmt {
        MacroStatement::LabelDefinition(name) => {
            asm.extend([
                Asm::Mark(*label_stack.get(name.ident()).unwrap()),
                Asm::Op(Opcode::JUMPDEST),
            ]);
            Ok(())
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
                    &target,
                    args.0
                        .iter()
                        .map(|arg| instruction_to_asm(globals, &current_args, label_stack, arg))
                        .collect(),
                    mark_tracker,
                    label_stack,
                    asm,
                )
            }
            _ => Err(format!(
                "Compilation not yet implemented for this invocation type `{:?}`",
                invoke
            )),
        },
        MacroStatement::Instruction(ref i) => {
            asm.push(instruction_to_asm(globals, &current_args, label_stack, i));
            Ok(())
        }
    });

    label_stack.leave_context();

    res
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
pub struct IncludedMacro {
    start_id: usize,
    end_id: usize,
    code: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct CompileGlobals<'src, 'ast> {
    pub allow_push0: bool,
    pub defs: BTreeMap<&'src str, &'ast Definition<'src>>,
    pub constants: BTreeMap<&'src str, U256>,
    pub referenced_macros: BTreeMap<&'src str, IncludedMacro>,
}

impl<'src, 'ast> CompileGlobals<'src, 'ast> {
    pub fn new(allow_push0: bool, defs: BTreeMap<&'src str, &'ast Definition<'src>>) -> Self {
        let constants = evalute_constants(&defs);
        Self {
            allow_push0,
            defs,
            constants,
            referenced_macros: BTreeMap::new(),
        }
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
