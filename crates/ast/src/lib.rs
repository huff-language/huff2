mod parser;

pub use parser::parse;

lalrpop_util::lalrpop_mod!(grammar);

use alloy_dyn_abi::DynSolType;
use alloy_primitives::U256;
use evm_glue::opcodes::Opcode;

pub struct Root<'src>(pub Box<[HuffDefinition<'src>]>);

#[derive(Debug, PartialEq, Eq)]
pub enum BuiltinInvoke<'src> {
    TableStart(&'src str),
    TableSize(&'src str),
    CodeSize(&'src str),
    CodeOffset(&'src str),
    FuncSig(&'src str),
    EventSig(&'src str),
}

#[derive(Debug, PartialEq, Eq)]
pub enum MacroExpr<'src> {
    Op(Opcode),
    MacroArgReference(&'src str),
}

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq)]
pub struct Jumptable<'src> {
    pub name: &'src str,
    pub size: u8,
    pub labels: Box<[&'src str]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Macro<'src> {
    pub name: &'src str,
    pub body: Box<[MacroStatement<'src>]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AbiFunction<'src> {
    pub name: &'src str,
    pub args: Box<[DynSolType]>,
    pub rets: Box<[DynSolType]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AbiEvent<'src> {
    pub name: &'src str,
    pub args: Box<[DynSolType]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AbiError<'src> {
    pub name: &'src str,
    pub args: Box<[DynSolType]>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HuffDefinition<'src> {
    Macro(Macro<'src>),
    Constant { name: &'src str, value: U256 },
    Jumptable(Jumptable<'src>),
    Codetable { name: &'src str, data: Box<[u8]> },
    AbiFunction(AbiFunction<'src>),
    AbiEvent(AbiEvent<'src>),
    AbiError(AbiError<'src>),
}
