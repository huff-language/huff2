use alloy_dyn_abi::DynSolType;
use alloy_primitives::U256;
use chumsky::span::SimpleSpan;
use evm_glue::opcodes::Opcode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Root<'src>(pub Box<[RootSection<'src>]>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootSection<'src> {
    Definition(Definition<'src>),
    Include(Spanned<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Definition<'src> {
    Macro(Macro<'src>),
    Constant {
        name: Spanned<&'src str>,
        expr: Spanned<ConstExpr>,
    },
    Jumptable(Spanned<Jumptable<'src>>),
    CodeTable(CodeTable<'src>),
    SolFunction(SolFunction<'src>),
    SolEvent(SolEvent<'src>),
    SolError(SolError<'src>),
}

pub trait IdentifiableNode<'a> {
    fn spanned(&self) -> &Spanned<&'a str>;

    fn ident(&self) -> &'a str {
        self.spanned().0
    }

    fn span(&self) -> Span {
        self.spanned().1
    }
}

impl<'src> IdentifiableNode<'src> for Definition<'src> {
    fn spanned(&self) -> &Spanned<&'src str> {
        match self {
            Self::Macro(m) => &m.name,
            Self::Constant { name, .. } => name,
            Self::Jumptable(jt) => &jt.0.name,
            Self::CodeTable(ct) => &ct.name,
            Self::SolEvent(e) => &e.name,
            Self::SolError(e) => &e.name,
            Self::SolFunction(f) => &f.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Macro<'src> {
    pub name: Spanned<&'src str>,
    pub args: Spanned<Box<[Spanned<&'src str>]>>,
    pub takes_returns: Option<(Spanned<usize>, Spanned<usize>)>,
    pub body: Box<[MacroStatement<'src>]>,
}

impl<'src> IdentifiableNode<'src> for Macro<'src> {
    fn spanned(&self) -> &Spanned<&'src str> {
        &self.name
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

impl Instruction<'_> {
    pub fn get_span(&self) -> Span {
        match self {
            Self::Op(s) => s.1,
            Self::VariablePush(s) => s.1,
            Self::LabelReference(name)
            | Self::MacroArgReference(name)
            | Self::ConstantReference(name) => name.1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invoke<'src> {
    Macro {
        name: Spanned<&'src str>,
        args: Spanned<Box<[Instruction<'src>]>>,
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
pub struct CodeTable<'src> {
    pub name: Spanned<&'src str>,
    pub data: Box<[u8]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jumptable<'src> {
    pub name: Spanned<&'src str>,
    pub label_size: u8,
    pub labels: Box<[Spanned<&'src str>]>,
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
    fn spanned(&self) -> &Spanned<&'src str> {
        self
    }
}
