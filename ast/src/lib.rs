use alloy_dyn_abi::DynSolType;
use evm_glue::opcodes::Opcode;

#[derive(Debug)]
pub enum BuiltinInvoke<'src> {
    TableStart(&'src str),
    TableSize(&'src str),
    CodeSize(&'src str),
    CodeOffset(&'src str),
    FuncSig(&'src str),
    EventSig(&'src str),
}

#[derive(Debug)]
pub enum MacroExpr<'src> {
    Op(Opcode),
    MacroArgReference(&'src str),
}

#[derive(Debug)]
pub enum MacroStatement<'src> {
    LabelDefinition(&'src str),
    LabelReference(&'src str),
    MacroInvoke {
        name: &'src str,
        args: Box<[MacroExpr<'src>]>,
    },
    BuiltinInvoke(BuiltinInvoke<'src>),
    Expr(MacroExpr<'src>),
}

#[derive(Debug)]
pub struct Jumptable<'src> {
    pub name: &'src str,
    pub size: u8,
    pub labels: Box<[&'src str]>,
}

#[derive(Debug)]
pub struct Macro<'src> {
    pub name: &'src str,
    pub body: Box<[MacroStatement<'src>]>,
}

#[derive(Debug)]
pub struct AbiFunction<'src> {
    pub name: &'src str,
    pub args: Box<[DynSolType]>,
}

#[derive(Debug)]
pub struct AbiEvent<'src> {
    pub name: &'src str,
    pub args: Box<[DynSolType]>,
}

#[derive(Debug)]
pub enum HuffDefinition<'src> {
    Macro(Macro<'src>),
    Jumptable(Jumptable<'src>),
    Codetable { name: &'src str, data: Box<[u8]> },
    AbiFunction(AbiFunction<'src>),
    AbiEvent(AbiEvent<'src>),
}
