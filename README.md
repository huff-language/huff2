# Huff 2

Huff2 is the successor of the [huff-rs](https://github.com/huff-language/huff-rs) compiler. At a
high-level its goals are:
- to be a maintanable and clean implementation of the language
- generalize and simplify language features
- make it more feasible / scalable to write extremely low-level, optimized code
- EOF?

## Roadmap

1. (🚧 Current WIP) _mostly_ backwards compatible compiler, with good error handling and error messages
   - [x] Grammar + Lexer + Parser
   - [ ] Parser error recovery
       - [ ] Basic invalid tokens
       - [ ] Unclosed brackets
       - [ ] Invalid nested definitions
   - [ ] Semantic Analysis
       - [ ] Global map of top-level definitions
       - [ ] Macro & macro arg reference validation
       - [ ] Pretty errors
       - [ ] Track tree of macro invocations & label definitions
       - [ ] Resolve label references & error on ambiguous label references
       - [ ] Resolve set of "objects" to include in compilation (code tables, jump tables, macros)
       - [ ] Resolve table labels based on scope in which they're referenced
   - [ ] CLI & Compilation
       - [ ] Runtime/Initcode flags (`-r` vs. `-b`)
       - [ ] Default constructor generation when `-b` and no explicit constructor set
       - [ ] Ability to output sizes for contracts
2. Add small QoL features:
    - [ ] Pushed tables (jump tables that are inlined as `PUSH<X>` instructions)
    - [ ] Make redundant syntax like `#define` optional
    - [ ] Decimal & binary literals
    - [ ] Make macros hygenic by making labels scoped
    - [ ] Introduce "function" labels for explicit out-of-scope jumping
3. Add EOF v1 support
4. ??? Expand the language even further (potential ideas)
    - BALLS integration for automated stack scheduling
    - Comp-time expressions
        - basic operators (`+ - / * ^ | & !`) on literals and comp-time builtin outputs
        - bytes/string literals
