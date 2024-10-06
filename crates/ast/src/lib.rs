mod parser;

pub use parser::parse;

lalrpop_util::lalrpop_mod!(grammar);

use alloy_dyn_abi::DynSolType;
use alloy_primitives::U256;
use revm_interpreter::opcode::OpCode;

pub struct Root<'src>(pub Box<[HuffDefinition<'src>]>);

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction<'src> {
    Op(OpCode),
    Push(OpCode, U256),
    PushAuto(U256),
    LabelDefinition(&'src str),
    LabelReference(&'src str),
    MacroArgReference(&'src str),
    ConstantReference(&'src str),
    Invoke(Invoke<'src>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Invoke<'src> {
    Macro { name: &'src str, args: Box<[U256]> },
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
    pub takes: usize,
    pub returns: usize,
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
