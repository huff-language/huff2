use alloy_dyn_abi::DynSolType;
use alloy_primitives::U256;
use chumsky::span::SimpleSpan;
use evm_glue::opcodes::Opcode;

#[derive(Debug, PartialEq, Eq)]
pub struct Root<'src>(pub Box<[RootSection<'src>]>);

#[derive(Debug, PartialEq, Eq)]
pub enum RootSection<'src> {
    Definition(Definition<'src>),
    Include(Spanned<String>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Definition<'src> {
    Macro(Macro<'src>),
    Constant {
        name: Spanned<&'src str>,
        expr: Spanned<ConstExpr>,
    },
    Jumptable(Jumptable<'src>),
    Table {
        name: Spanned<&'src str>,
        data: Box<[u8]>,
    },
    SolFunction(SolFunction<'src>),
    SolEvent(SolEvent<'src>),
    SolError(SolError<'src>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Macro<'src> {
    pub name: Spanned<&'src str>,
    pub args: Box<[Spanned<&'src str>]>,
    pub takes_returns: Option<(Spanned<usize>, Spanned<usize>)>,
    pub body: Box<[MacroStatement<'src>]>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConstExpr {
    Value(U256),
    FreeStoragePointer,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MacroStatement<'src> {
    LabelDefinition(Spanned<&'src str>),
    Instruction(Instruction<'src>),
    Invoke(Invoke<'src>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction<'src> {
    Op(Spanned<Opcode>),
    LabelReference(Spanned<&'src str>),
    MacroArgReference(Spanned<&'src str>),
    ConstantReference(Spanned<&'src str>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Invoke<'src> {
    Macro {
        name: Spanned<&'src str>,
        args: Box<[Instruction<'src>]>,
    },
    BuiltinTableStart(Spanned<&'src str>),
    BuiltinTableSize(Spanned<&'src str>),
    BuiltinCodeSize(Spanned<&'src str>),
    BuiltinCodeOffset(Spanned<&'src str>),
    BuiltinFuncSig(Spanned<&'src str>),
    BuiltinEventHash(Spanned<&'src str>),
    BuiltinError(Spanned<&'src str>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Jumptable<'src> {
    pub name: Spanned<&'src str>,
    pub size: u8,
    pub labels: Box<[&'src str]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SolFunction<'src> {
    pub name: Spanned<&'src str>,
    pub args: Box<[Spanned<DynSolType>]>,
    pub rets: Box<[Spanned<DynSolType>]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SolEvent<'src> {
    pub name: Spanned<&'src str>,
    pub args: Box<[Spanned<DynSolType>]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SolError<'src> {
    pub name: Spanned<&'src str>,
    pub args: Box<[Spanned<DynSolType>]>,
}

/// A span.
pub type Span = SimpleSpan<usize>;

/// A spanned value.
pub type Spanned<T> = (T, Span);
