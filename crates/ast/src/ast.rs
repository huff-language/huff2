use alloy_dyn_abi::DynSolType;
use alloy_primitives::U256;
use chumsky::span::SimpleSpan;
use evm_glue::opcodes::Opcode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Root<'src>(pub Box<[RootSection<'src>]>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootSection<'src> {
    Definition(Definition<'src>),
    Include(Spanned<&'src str>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub trait IdentifiableNode<'a> {
    fn ident(&self) -> &'a str;
}

impl<'src> IdentifiableNode<'src> for Definition<'src> {
    fn ident(&self) -> &'src str {
        match self {
            Self::Macro(m) => m.name.0,
            Self::Constant { name, .. } => name.0,
            Self::Jumptable(jt) => jt.name.0,
            Self::Table { name, .. } => name.0,
            Self::SolEvent(e) => e.name.0,
            Self::SolError(e) => e.name.0,
            Self::SolFunction(f) => f.name.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Macro<'src> {
    pub name: Spanned<&'src str>,
    pub args: Box<[Spanned<&'src str>]>,
    pub takes_returns: Option<(Spanned<usize>, Spanned<usize>)>,
    pub body: Box<[MacroStatement<'src>]>,
}

impl<'src> IdentifiableNode<'src> for Macro<'src> {
    fn ident(&self) -> &'src str {
        self.name.ident()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstExpr {
    Value(U256),
    FreeStoragePointer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacroStatement<'src> {
    LabelDefinition(Spanned<&'src str>),
    Instruction(Instruction<'src>),
    Invoke(Invoke<'src>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction<'src> {
    Op(Spanned<Opcode>),
    VariablePush(Spanned<U256>),
    LabelReference(Spanned<&'src str>),
    MacroArgReference(Spanned<&'src str>),
    ConstantReference(Spanned<&'src str>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jumptable<'src> {
    pub name: Spanned<&'src str>,
    pub size: u8,
    pub labels: Box<[&'src str]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolFunction<'src> {
    pub name: Spanned<&'src str>,
    pub args: Box<[Spanned<DynSolType>]>,
    pub rets: Box<[Spanned<DynSolType>]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolEvent<'src> {
    pub name: Spanned<&'src str>,
    pub args: Box<[Spanned<DynSolType>]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolError<'src> {
    pub name: Spanned<&'src str>,
    pub args: Box<[Spanned<DynSolType>]>,
}

/// A span.
pub type Span = SimpleSpan<usize>;

/// A spanned value.
pub type Spanned<T> = (T, Span);

impl<'src> IdentifiableNode<'src> for Spanned<&'src str> {
    fn ident(&self) -> &'src str {
        self.0
    }
}
