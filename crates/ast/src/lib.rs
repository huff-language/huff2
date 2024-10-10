mod error;
mod parser;

pub use error::Error;
pub use parser::parse;

lalrpop_util::lalrpop_mod!(grammar);

use alloy_dyn_abi::DynSolType;
use alloy_primitives::U256;
use evm_glue::opcodes::Opcode;

pub struct Root<'src>(pub Box<[HuffDefinition<'src>]>);

#[derive(Debug, PartialEq, Eq)]
pub enum MacroStatement<'src> {
    LabelDefinition(&'src str),
    Instruction(Instruction<'src>),
    Invoke(Invoke<'src>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction<'src> {
    Op(Opcode),
    LabelReference(&'src str),
    MacroArgReference(&'src str),
    ConstantReference(&'src str),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Invoke<'src> {
    Macro {
        name: &'src str,
        args: Box<[Instruction<'src>]>,
    },
    BuiltinTableStart(&'src str),
    BuiltinTableSize(&'src str),
    BuiltinCodeSize(&'src str),
    BuiltinCodeOffset(&'src str),
    BuiltinFuncSig(&'src str),
    BuiltinEventHash(&'src str),
    BuiltinError(&'src str),
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
    pub args: Box<[&'src str]>,
    pub takes_returns: Option<(usize, usize)>,
    pub body: Box<[Instruction<'src>]>,
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
