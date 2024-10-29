# Huff 2

Huff2 is the successor of the [huff-rs](https://github.com/huff-language/huff-rs) compiler written
in Rust. It comes with:
- better error handling
- clearer semantics (only 1 label def. per scope, checks for macro argument count match)
- push minimization (e.g. will use `PUSH1` instead of `PUSH2` if referencing a label with `PC < 256`)
- new builtins:
    - `__codeoffset(macro_name: MacroIdentifier)`

## Missing Features / TODOs

- [ ] Jump tables
    - [ ] parsing
    - [ ] builtins (`__tablestart`, `__tablesize`)
- [ ] Code tables
    - [ ] builtins (`__tablestart`, `__tablesize`)
- [ ] ABI builtins (`__FUNC_SIG`, `__EVEN_HASH`, `__ERROR`)
- [ ] Macro code inclusion builtins (`__codesize`, `__codeoffset`)
- [ ] Imports (`#include` statements)

## Why rewrite `huff-rs`?

The [`huff-rs`](https://github.com/huff-language/huff-rs) compiler was a passion project by pioneers
in the Huff community who aimed to create a better version of Zac's
[original typescript implementation](https://github.com/AztecProtocol/huff).

The initial developers of `huff-rs` were relatively new to compilers, choosing to write some of the
components like the lexer, parser & assembler themselves while adding novel compiler features like
the Huff testing framework that unfortunately didn't see a lot of usage.

Combined with a lot of the tech debt that accrued from the early days made us decide that it was
best to start fresh, using existing libraries to do as much of the heavy lifting as possible:
- [`chumsky`](https://github.com/zesterer/chumsky/) for lexing & parsing
- [`ariadne`](https://github.com/zesterer/ariadne) for pretty errors
- [`evm-glue`](https://github.com/philogy/evm-glue) for EVM assembly
- [`alloy`](https://alloy.rs/) for ABI types & parsing

This new foundation will allow bugs to be fixed more easily as well as allowing us to experiment
with our own novel compiler features. 😁

## Differences vs. `huff-rs`
### CLI Changes
The `-b`, `--bytecode`, `-r`, `--bin-runtime`, `-m`, `--alt-main`, `-t`, `--alt-constructor` have
been replaced in favor of a required positional argument indicating what macro to compile and
optional `-f` / `--default-constructor` flags to wrap the compiled result with a minimal default
constructor.

This was done to make the CLI simpler and clearer, you will always get a single output, the output
you ask for and nothing extra will be added without you asking for it.

A lot of other flags were not reimplemented either because they were not widely used or because we
just haven't gotten around to it. Raise an issue if you'd like to suggest a feature.

### Stricter Code Validation
The compiler will not validatate certain things that were not checked/simply allowed in the previous
compiler:
- duplicate label definitions in the same scope
- duplicate definitions (you could define multiple constants, macros, etc. with identical names)
- mismatching macro argument count

These errors were serious footguns that could easily go unnoticed when using the previous compiler.

### New Label & Jump Table Semantics

1. **Up Only**: Macros can only reference labels defined within them or the parents invoking them
2. **Shadowing:** Label definitions deeper down in a chain of invocations shadows previous definitions

**Examples:**

References resolve to the labels defined within the same macro:

```

#define macro INNER() = {
    target   ✅ Resolves ─┐
                          │
    0x1 0x1               │
    add                   │
    0x2                   │
    eq                    │
                          │
    target:  <────────────┘
}

```

Falls back to resolving to invoker's label:

```
#define macro MAIN() = takes(0) returns(0) {
    INNER() ────┐
    target: <───┘
}


#define macro INNER() = {
                ^
                │
    target   ✅ Resolves

    0x1 0x1
    add
    0x2
    eq
}
```

Resolution **does not** go down into invoked macros:

```
#define macro MAIN() = takes(0) returns(0) {
    INNER()
    target   ❌ Fails to Resolve
}

#define macro INNER() = {
    target:

    0x1 0x1
    add
    0x2
    eq
}
```

As you go down an invocation chain label definitions are added to a stack where the highest most
definition is resolved by references.

```
#define macro MAIN() = takes(0) returns(0) {
    INNER()
    target:  🟡 Shadowed by ────┐
}                               │
                                │
                                │
#define macro INNER() = {       │
    target   ✅ Resolves ─┐     │
                          │     │
    0x1 0x1               │     │
    add                   │     │
    0x2                   │     │
    eq                    │     │
                          │     │
    target:  <────────────┘ <───┘
}
```

