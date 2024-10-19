use huff_ast::{Definition, Instruction, Invoke, Macro, Spanned};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisError<'ast, 'src> {
    /// When two different definitions have the same.
    DefinitionNameCollision {
        collided: Box<[&'ast Definition<'src>]>,
        duplicate_name: &'src str,
    },
    RecursiveMacroInvocation {
        invocation_chain: Box<[(&'ast Macro<'src>, &'ast Invoke<'src>)]>,
    },
    ReferenceNotFound {
        scope: &'ast Macro<'src>,
        ref_type: &'static str,
        not_found: &'ast Instruction<'src>,
    },
    DefinitionNotFound {
        scope: &'ast Macro<'src>,
        def_type: &'static str,
        name: &'ast Spanned<&'src str>,
    },
    MacroArgumentCountMismatch {
        scope: &'ast Macro<'src>,
        args: &'ast [Instruction<'src>],
        target: &'ast Macro<'src>,
    },
    DuplicateLabelDefinition {
        scope: &'ast Macro<'src>,
        duplicates: Box<[&'ast Spanned<&'src str>]>,
    },
    DuplicateMacroArgDefinition {
        scope: &'ast Macro<'src>,
        duplicates: Box<[&'ast Spanned<&'src str>]>,
    },
    NotYetSupported {
        intent: String,
        span: Spanned<()>,
    },
}
