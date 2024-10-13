mod error;
mod parser;

pub use error::Error;
pub use parser::parse;

pub type Span = std::ops::Range<usize>;

lalrpop_util::lalrpop_mod!(
    #[allow(clippy::all)]
    grammar
);

use alloy_dyn_abi::DynSolType;
use alloy_primitives::U256;
use evm_glue::opcodes::Opcode;

#[derive(Debug, PartialEq, Eq)]
pub struct Root<'src>(pub Box<[Definition<'src>]>);

#[derive(Debug, PartialEq, Eq)]
pub enum Definition<'src> {
    Macro(Macro<'src>),
    Constant {
        name: (Span, &'src str),
        value: U256,
    },
    Jumptable(Jumptable<'src>),
    Codetable {
        name: (Span, &'src str),
        data: Box<[u8]>,
    },
    SolFunction(SolFunction<'src>),
    SolEvent(SolEvent<'src>),
    SolError(SolError<'src>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Macro<'src> {
    pub name: (Span, &'src str),
    pub args: Box<[(Span, &'src str)]>,
    pub takes_returns: Option<(usize, usize)>,
    pub body: Box<[(Span, MacroStatement<'src>)]>,
}

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
        name: (Span, &'src str),
        args: Box<[(Span, Instruction<'src>)]>,
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
    pub name: (Span, &'src str),
    pub size: u8,
    pub labels: Box<[&'src str]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SolFunction<'src> {
    pub name: &'src str,
    pub args: Box<[DynSolType]>,
    pub rets: Box<[DynSolType]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SolEvent<'src> {
    pub name: &'src str,
    pub args: Box<[DynSolType]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SolError<'src> {
    pub name: &'src str,
    pub args: Box<[DynSolType]>,
}
