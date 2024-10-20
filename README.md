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
   - [x] Semantic Analysis
   - [ ] CLI & Compilation
2. Add small QoL features:
    - [ ] Pushed tables (jump tables that are inlined as `PUSH<X>` instructions)
    - [ ] Make redundant syntax like `#define` optional
    - [x] Decimal & binary literals
    - [x] Make macros hygenic by making labels scoped
    - [ ] Better parser error recovery
3. ??? Expand the language even further (potential ideas)
    - BALLS integration for automated stack scheduling
    - Comp-time expressions
        - basic operators (`+ - / * ^ | & !`) on literals and comp-time builtin outputs
        - bytes/string literals
4. Add EOF v1 support
