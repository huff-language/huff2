use crate::{
    ast,
    lexer::{
        lex,
        Token::{self, *},
    },
    util::{u256_as_push, u256_as_push_data},
    Span, Spanned,
};
use alloy_dyn_abi::DynSolType;
use alloy_primitives::{hex::FromHex, Bytes, U256};
use chumsky::{
    error::Rich,
    extra,
    input::{Input, SpannedInput},
    primitive::{choice, just},
    recursive::recursive,
    select,
    span::SimpleSpan,
    IterParser, Parser as ChumskyParser,
};
use evm_glue::opcodes::Opcode;
use std::str::FromStr;

/// Parse the given source code string into AST.
///
/// # Arguments
///
/// * `src` - A string that holds the source code to be parsed.
pub fn parse(src: &str) -> Result<ast::Root<'_>, Vec<Rich<'_, Token<'_>>>> {
    let tokens = lex(src)?;

    let eoi: Span = SimpleSpan::new(src.len(), src.len());
    let tokens = tokens.as_slice().spanned(eoi);
    let ast = root()
        .parse(tokens)
        .into_result()
        .map_err(|errs| errs.into_iter().map(|e| e.into_owned()).collect::<Vec<_>>())
        .map_err(|e| e)?;

    Ok(ast)
}

type ParserInput<'tokens, 'src> = SpannedInput<Token<'src>, Span, &'tokens [Spanned<Token<'src>>]>;

trait Parser<'tokens, 'src: 'tokens, T>:
    ChumskyParser<'tokens, ParserInput<'tokens, 'src>, T, extra::Err<Rich<'tokens, Token<'src>, Span>>>
{
}
impl<'tokens, 'src: 'tokens, P, T> Parser<'tokens, 'src, T> for P where
    P: ChumskyParser<
        'tokens,
        ParserInput<'tokens, 'src>,
        T,
        extra::Err<Rich<'tokens, Token<'src>, Span>>,
    >
{
}

fn root<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ast::Root<'src>> {
    definition()
        .repeated()
        .collect::<Vec<_>>()
        .map(|defs| ast::Root(defs.into_boxed_slice()))
}

fn definition<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ast::Definition<'src>> {
    just(Keyword("define")).ignore_then(choice((
        r#macro(),
        constant(),
        table(),
        sol_function(),
        sol_event(),
        sol_error(),
    )))
}

fn r#macro<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ast::Definition<'src>> {
    let macro_args = ident().separated_by(just(Punct(','))).collect::<Vec<_>>();

    just(Ident("macro"))
        .ignore_then(ident())
        .then_ignore(just(Punct('(')))
        .then(macro_args)
        .then_ignore(just(Punct(')')))
        .then_ignore(just(Punct('=')))
        .then(
            just(Ident("takes"))
                .ignore_then(just(Punct('(')))
                .ignore_then(dec())
                .then_ignore(just(Punct(')')))
                .then_ignore(just(Ident("returns")))
                .then_ignore(just(Punct('(')))
                .then(dec())
                .then_ignore(just(Punct(')')))
                .or_not(),
        )
        .then_ignore(just(Punct('{')))
        .then(macro_statement().repeated().collect::<Vec<_>>())
        .then_ignore(just(Punct('}')))
        .map(|(((name, args), takes_returns), body)| ast::Macro {
            name,
            args: args.into_boxed_slice(),
            takes_returns,
            body: body.into_boxed_slice(),
        })
        .map(ast::Definition::Macro)
}

fn macro_statement<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ast::MacroStatement<'src>>
{
    let label = ident()
        .then_ignore(just(Punct(':')))
        .map(ast::MacroStatement::LabelDefinition);
    let instruction = instruction().map(ast::MacroStatement::Instruction);
    let invoke = invoke().map(ast::MacroStatement::Invoke);

    choice((label, invoke, instruction))
}

fn instruction<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ast::Instruction<'src>> {
    let push_auto = word().map(|(value, span)| (ast::Instruction::Op((u256_as_push(value), span))));

    let push = select! {
        Ident("push1") => 1,
        Ident("push2") => 2,
        Ident("push3") => 3,
        Ident("push4") => 4,
        Ident("push5") => 5,
        Ident("push6") => 6,
        Ident("push7") => 7,
        Ident("push8") => 8,
        Ident("push9") => 9,
        Ident("push10") => 10,
        Ident("push11") => 11,
        Ident("push12") => 12,
        Ident("push13") => 13,
        Ident("push14") => 14,
        Ident("push15") => 15,
        Ident("push16") => 16,
        Ident("push17") => 17,
        Ident("push18") => 18,
        Ident("push19") => 19,
        Ident("push20") => 20,
        Ident("push21") => 21,
        Ident("push22") => 22,
        Ident("push23") => 23,
        Ident("push24") => 24,
        Ident("push25") => 25,
        Ident("push26") => 26,
        Ident("push27") => 27,
        Ident("push28") => 28,
        Ident("push29") => 29,
        Ident("push30") => 30,
        Ident("push31") => 31,
        Ident("push32") => 32,
    }
    .then(word())
    .map(|(n, (value, span))| {
        (
            match n {
                1 => Opcode::PUSH1(u256_as_push_data::<1>(value).unwrap()),
                2 => Opcode::PUSH2(u256_as_push_data::<2>(value).unwrap()),
                3 => Opcode::PUSH3(u256_as_push_data::<3>(value).unwrap()),
                4 => Opcode::PUSH4(u256_as_push_data::<4>(value).unwrap()),
                5 => Opcode::PUSH5(u256_as_push_data::<5>(value).unwrap()),
                6 => Opcode::PUSH6(u256_as_push_data::<6>(value).unwrap()),
                7 => Opcode::PUSH7(u256_as_push_data::<7>(value).unwrap()),
                8 => Opcode::PUSH8(u256_as_push_data::<8>(value).unwrap()),
                9 => Opcode::PUSH9(u256_as_push_data::<9>(value).unwrap()),
                10 => Opcode::PUSH10(u256_as_push_data::<10>(value).unwrap()),
                11 => Opcode::PUSH11(u256_as_push_data::<11>(value).unwrap()),
                12 => Opcode::PUSH12(u256_as_push_data::<12>(value).unwrap()),
                13 => Opcode::PUSH13(u256_as_push_data::<13>(value).unwrap()),
                14 => Opcode::PUSH14(u256_as_push_data::<14>(value).unwrap()),
                15 => Opcode::PUSH15(u256_as_push_data::<15>(value).unwrap()),
                16 => Opcode::PUSH16(u256_as_push_data::<16>(value).unwrap()),
                17 => Opcode::PUSH17(u256_as_push_data::<17>(value).unwrap()),
                18 => Opcode::PUSH18(u256_as_push_data::<18>(value).unwrap()),
                19 => Opcode::PUSH19(u256_as_push_data::<19>(value).unwrap()),
                20 => Opcode::PUSH20(u256_as_push_data::<20>(value).unwrap()),
                21 => Opcode::PUSH21(u256_as_push_data::<21>(value).unwrap()),
                22 => Opcode::PUSH22(u256_as_push_data::<22>(value).unwrap()),
                23 => Opcode::PUSH23(u256_as_push_data::<23>(value).unwrap()),
                24 => Opcode::PUSH24(u256_as_push_data::<24>(value).unwrap()),
                25 => Opcode::PUSH25(u256_as_push_data::<25>(value).unwrap()),
                26 => Opcode::PUSH26(u256_as_push_data::<26>(value).unwrap()),
                27 => Opcode::PUSH27(u256_as_push_data::<27>(value).unwrap()),
                28 => Opcode::PUSH28(u256_as_push_data::<28>(value).unwrap()),
                29 => Opcode::PUSH29(u256_as_push_data::<29>(value).unwrap()),
                30 => Opcode::PUSH30(u256_as_push_data::<30>(value).unwrap()),
                31 => Opcode::PUSH31(u256_as_push_data::<31>(value).unwrap()),
                32 => Opcode::PUSH32(u256_as_push_data::<32>(value).unwrap()),
                _ => unreachable!(),
            },
            span,
        )
    })
    .map(ast::Instruction::Op);

    let op = ident().map(|(ident, span)| {
        if let Ok(op) = Opcode::from_str(ident) {
            ast::Instruction::Op((op, span))
        } else {
            ast::Instruction::LabelReference((ident, span))
        }
    });
    let macro_arg_ref = just(Punct('<'))
        .ignore_then(ident())
        .then_ignore(just(Punct('>')))
        .map(ast::Instruction::MacroArgReference);
    let constant_ref = just(Punct('['))
        .ignore_then(ident())
        .then_ignore(just(Punct(']')))
        .map(ast::Instruction::ConstantReference);

    choice((push_auto, push, op, macro_arg_ref, constant_ref))
}

fn invoke<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ast::Invoke<'src>> {
    let invoke_macro_args = just(Punct('('))
        .ignore_then(
            instruction()
                .separated_by(just(Punct(',')))
                .collect::<Vec<_>>(),
        )
        .then_ignore(just(Punct(')')))
        .map(|args| args.into_boxed_slice());

    let invoke_macro = ident()
        .then(invoke_macro_args)
        .map(|(name, args)| ast::Invoke::Macro { name, args });

    let invoke_builtin = |name, constructor: fn((_, Span)) -> ast::Invoke<'src>| {
        just(Ident(name))
            .ignore_then(just(Punct('(')))
            .ignore_then(ident())
            .then_ignore(just(Punct(')')))
            .map(constructor)
    };

    choice((
        invoke_builtin("__tablestart", ast::Invoke::BuiltinTableStart),
        invoke_builtin("__tablesize", ast::Invoke::BuiltinTableSize),
        invoke_builtin("__codesize", ast::Invoke::BuiltinCodeSize),
        invoke_builtin("__codeoffset", ast::Invoke::BuiltinCodeOffset),
        invoke_builtin("__FUNC_SIG", ast::Invoke::BuiltinFuncSig),
        invoke_builtin("__EVENT_HASH", ast::Invoke::BuiltinEventHash),
        invoke_builtin("__ERROR", ast::Invoke::BuiltinError),
        invoke_macro,
    ))
}

fn constant<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ast::Definition<'src>> {
    just(Ident("constant"))
        .ignore_then(ident())
        .then_ignore(just(Punct('=')))
        .then(word())
        .map(|(name, (value, _))| ast::Definition::Constant { name, value })
}

fn table<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ast::Definition<'src>> {
    just(Ident("table"))
        .ignore_then(ident())
        .then_ignore(just(Punct('{')))
        .then(code().repeated().collect::<Vec<_>>())
        .then_ignore(just(Punct('}')))
        .map(|(name, code)| ast::Definition::Table {
            name,
            data: code
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        })
}

fn sol_function<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ast::Definition<'src>> {
    just(Ident("function"))
        .ignore_then(ident())
        .then(sol_type_list())
        .then(
            choice((just(Ident("public")), just(Ident("external"))))
                .then_ignore(choice((just(Ident("view")), just(Ident("pure")))).or_not())
                .then_ignore(choice((just(Ident("payable")), just(Ident("nonpayable")))).or_not())
                .or_not()
                .ignore_then(just(Ident("returns")))
                .ignore_then(sol_type_list())
                .or_not(),
        )
        .map(|((name, args), rets)| {
            let rets = rets.unwrap_or(Box::new([]));
            ast::Definition::SolFunction(ast::SolFunction { name, args, rets })
        })
}

fn sol_event<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ast::Definition<'src>> {
    just(Ident("event"))
        .ignore_then(ident())
        .then(sol_type_list())
        .map(|(name, args)| ast::Definition::SolEvent(ast::SolEvent { name, args }))
}

fn sol_error<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ast::Definition<'src>> {
    just(Ident("error"))
        .ignore_then(ident())
        .then(sol_type_list())
        .map(|(name, args)| ast::Definition::SolError(ast::SolError { name, args }))
}

fn sol_type_list<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, Box<[Spanned<DynSolType>]>>
{
    just(Punct('('))
        .ignore_then(
            sol_type()
                .separated_by(just(Punct(',')))
                .collect::<Vec<_>>(),
        )
        .then_ignore(just(Punct(')')))
        .map(|args| args.into_boxed_slice())
}

fn sol_type<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, Spanned<DynSolType>> {
    recursive(|sol_raw_type| {
        let sol_raw_primitive_type = ident().map(|(typ, _)| typ.to_string()).boxed();

        let sol_raw_tuple_type = just(Punct('('))
            .ignore_then(
                sol_raw_type
                    .separated_by(just(Punct(',')))
                    .collect::<Vec<_>>(),
            )
            .then_ignore(just(Punct(')')))
            .map(|types| {
                let mut result = "(".to_string();
                let types = types.into_iter().collect::<Vec<_>>().join(",");
                result.push_str(&types);
                result.push(')');
                result
            })
            .boxed();

        choice((sol_raw_primitive_type, sol_raw_tuple_type))
            .then(
                just(Punct('['))
                    .ignore_then(dec().or_not())
                    .then_ignore(just(Punct(']')))
                    .or_not(),
            )
            .then_ignore(ident().or_not())
            .map(|(typ, slice)| {
                let mut result = typ;
                if let Some(size) = slice {
                    result.push('[');
                    if let Some((n, _span)) = size {
                        result.push_str(&n.to_string());
                    }
                    result.push(']');
                }
                result
            })
            .boxed()
    })
    .try_map_with(|typ, ex| {
        DynSolType::parse(&typ)
            .map(|typ| (typ, ex.span()))
            .map_err(|e| Rich::custom(ex.span(), e))
    })
}

fn ident<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, Spanned<&'src str>> {
    select! {Ident(s) => s}.map_with(|s, ex| (s, ex.span()))
}

fn dec<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, Spanned<usize>> {
    select! {Dec(s) => s.parse::<usize>().unwrap()}.map_with(|s, ex| (s, ex.span()))
}

fn word<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, Spanned<U256>> {
    select! {
        Hex(s) => U256::from_str_radix(&s[2..], 16),
        Bin(s) => U256::from_str_radix(&s[2..], 2),
        Dec(s) => U256::from_str_radix(s, 10)
    }
    .try_map_with(|value, ex| value.map_err(|_e| Rich::custom(ex.span(), "word overflows")))
    .map_with(|value, ex| (value, ex.span()))
}

fn code<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, Vec<u8>> {
    select! {
        Hex(s) => Bytes::from_hex(s)
    }
    .try_map_with(|code, ex| code.map_err(|_e| Rich::custom(ex.span(), "odd length")))
    .map(|code| code.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::uint;
    use chumsky::input::Input;

    /// Macro to assert that a parser successfully parses a given set of tokens
    /// and produces the expected result.
    ///
    /// # Arguments
    ///
    /// * `$parser` - The parser to be tested.
    /// * `$tokens` - A collection of tokens to be parsed.
    /// * `$expected` - The expected result after parsing.
    macro_rules! assert_ok {
        ($parser:expr, $tokens:expr, $expected:expr) => {
            let tokens: Vec<(Token<'_>, SimpleSpan)> = $tokens
                .into_iter()
                .map(|tok| (tok.clone(), SimpleSpan::new(0, 0)))
                .collect();
            assert_eq!(
                $parser
                    .parse(tokens.as_slice().spanned(SimpleSpan::new(0, 0)))
                    .into_result(),
                Ok($expected),
            );
        };
    }

    /// Macro to assert that a parser returns an expected error when parsing a
    /// given set of tokens.
    ///
    /// # Arguments
    ///
    /// * `$parser` - The parser to be tested.
    /// * `$tokens` - A collection of tokens to be parsed.
    /// * `$expected` - The expected error message after parsing.
    macro_rules! assert_err {
        ($parser:expr, $tokens:expr, $expected:expr) => {
            let tokens: Vec<(Token<'_>, SimpleSpan)> = $tokens
                .into_iter()
                .map(|tok| (tok.clone(), SimpleSpan::new(0, 0)))
                .collect();
            let expected = vec![Rich::custom(SimpleSpan::new(0, 0), $expected)];
            assert_eq!(
                $parser
                    .parse(tokens.as_slice().spanned(SimpleSpan::new(0, 0)))
                    .into_result(),
                Err(expected),
            );
        };
    }

    #[test]
    fn parse_word() {
        let span: Span = SimpleSpan::new(0, 0);

        assert_ok!(word(), vec![Hex("0x0")], (U256::ZERO, span));
        assert_ok!(word(), vec![Hex("0x1")], (uint!(1_U256), span));
        assert_ok!(word(), vec![Bin("0b10")], (uint!(2_U256), span));
        assert_ok!(word(), vec![Dec("2")], (uint!(2_U256), span));
        assert_ok!(
            word(),
            vec![Hex("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")],
            (U256::MAX, span)
        );
        assert_err!(
            word(),
            vec![Hex("0x10000000000000000000000000000000000000000000000000000000000000000")],
            "word overflows"
        );
    }

    #[test]
    fn parse_code() {
        assert_ok!(code(), vec![Hex("0xc0de")], vec![0xc0, 0xde]);
        assert_err!(code(), vec![Hex("0x0")], "odd length");
    }

    #[test]
    fn parse_macro() {
        let span: Span = SimpleSpan::new(0, 0);
        assert_ok!(
            r#macro(),
            vec![
                Ident("macro"),
                Ident("MAIN"),
                Punct('('),
                Punct(')'),
                Punct('='),
                Punct('{'),
                Punct('}')
            ],
            ast::Definition::Macro(ast::Macro {
                name: ("MAIN", span),
                args: Box::new([]),
                takes_returns: None,
                body: Box::new([])
            })
        );
        assert_ok!(
            r#macro(),
            vec![
                Ident("macro"),
                Ident("READ_ADDRESS"),
                Punct('('),
                Ident("offset"),
                Punct(')'),
                Punct('='),
                Ident("takes"),
                Punct('('),
                Dec("0"),
                Punct(')'),
                Ident("returns"),
                Punct('('),
                Dec("1"),
                Punct(')'),
                Punct('{'),
                Ident("stop"),
                Punct('}')
            ],
            ast::Definition::Macro(ast::Macro {
                name: ("READ_ADDRESS", span),
                args: Box::new([("offset", span)]),
                takes_returns: Some(((0, span), (1, span))),
                body: Box::new([ast::MacroStatement::Instruction(ast::Instruction::Op((
                    Opcode::STOP,
                    span
                )))]),
            })
        );
    }

    #[test]
    fn parse_macro_statement() {
        let span: Span = SimpleSpan::new(0, 0);

        assert_ok!(
            macro_statement(),
            vec![Ident("x"), Punct(':')],
            ast::MacroStatement::LabelDefinition(("x", span))
        );
        assert_ok!(
            macro_statement(),
            vec![Ident("__tablestart"), Punct('('), Ident("TABLE"), Punct(')')],
            ast::MacroStatement::Invoke(ast::Invoke::BuiltinTableStart(("TABLE", span)))
        );
        assert_ok!(
            macro_statement(),
            vec![Ident("READ_ADDRESS"), Punct('('), Hex("0x4"), Punct(')')],
            ast::MacroStatement::Invoke(ast::Invoke::Macro {
                name: ("READ_ADDRESS", span),
                args: Box::new([ast::Instruction::Op((Opcode::PUSH1([0x04]), span))])
            })
        );
    }

    #[test]
    fn parse_instruction() {
        let span: Span = SimpleSpan::new(0, 0);

        assert_ok!(
            instruction(),
            vec![Ident("add")],
            ast::Instruction::Op((Opcode::ADD, span))
        );
        assert_ok!(
            instruction(),
            vec![Hex("0x1")],
            ast::Instruction::Op((Opcode::PUSH1([0x01]), span))
        );
        assert_ok!(
            instruction(),
            vec![Ident("push2"), Hex("0x1")],
            ast::Instruction::Op((Opcode::PUSH2([0x00, 0x01]), span))
        );
        assert_ok!(
            instruction(),
            vec![Ident("x")],
            ast::Instruction::LabelReference(("x", span))
        );
        assert_ok!(
            instruction(),
            vec![Punct('<'), Ident("x"), Punct('>')],
            ast::Instruction::MacroArgReference(("x", span))
        );
        assert_ok!(
            instruction(),
            vec![Punct('['), Ident("x"), Punct(']')],
            ast::Instruction::ConstantReference(("x", span))
        );
    }

    #[test]
    fn parse_constant() {
        let span: Span = SimpleSpan::new(0, 0);

        assert_ok!(
            constant(),
            vec![Ident("constant"), Ident("TEST"), Punct('='), Hex("0x1")],
            ast::Definition::Constant {
                name: ("TEST", span),
                value: uint!(1_U256)
            }
        );
    }

    #[test]
    fn parse_table() {
        let span: Span = SimpleSpan::new(0, 0);

        assert_ok!(
            table(),
            vec![Ident("table"), Ident("TEST"), Punct('{'), Hex("0xc0de"), Punct('}')],
            ast::Definition::Table {
                name: ("TEST", span),
                data: Box::new([0xc0, 0xde])
            }
        );
        assert_ok!(
            table(),
            vec![
                Ident("table"),
                Ident("TEST"),
                Punct('{'),
                Hex("0xc0de"),
                Hex("0xcc00ddee"),
                Punct('}')
            ],
            ast::Definition::Table {
                name: ("TEST", span),
                data: Box::new([0xc0, 0xde, 0xcc, 0x00, 0xdd, 0xee])
            }
        );
    }

    #[test]
    fn parse_sol_type() {
        let span: Span = SimpleSpan::new(0, 0);

        assert_ok!(
            sol_type(),
            vec![Ident("address"),],
            (DynSolType::parse("address").unwrap(), span)
        );
        assert_ok!(
            sol_type(),
            vec![Ident("address"), Ident("token")],
            (DynSolType::parse("address").unwrap(), span)
        );
        assert_ok!(
            sol_type(),
            vec![Ident("address"), Punct('['), Punct(']')],
            (DynSolType::parse("address[]").unwrap(), span)
        );
        assert_ok!(
            sol_type(),
            vec![Ident("address"), Punct('['), Dec("3"), Punct(']'),],
            (DynSolType::parse("address[3]").unwrap(), span)
        );
        assert_ok!(
            sol_type(),
            vec![
                Punct('('),
                Ident("address"),
                Ident("to"),
                Punct(','),
                Ident("uint256"),
                Ident("amount"),
                Punct(')'),
                Punct('['),
                Punct(']'),
            ],
            (DynSolType::parse("(address,uint256)[]").unwrap(), span)
        );
        assert_ok!(
            sol_type(),
            vec![
                Punct('('),
                Ident("address"),
                Punct(','),
                Punct('('),
                Ident("address"),
                Punct(','),
                Ident("uint256"),
                Punct(')'),
                Punct('['),
                Punct(']'),
                Punct(')'),
                Punct('['),
                Punct(']'),
            ],
            (
                DynSolType::parse("(address,(address,uint256)[])[]").unwrap(),
                span
            )
        );
    }

    #[test]
    fn parse_sol_type_list() {
        let span: Span = SimpleSpan::new(0, 0);

        assert_ok!(
            sol_type_list(),
            vec![Punct('('), Ident("address"), Punct(','), Ident("uint256"), Punct(')')],
            vec![
                (DynSolType::parse("address").unwrap(), span),
                (DynSolType::parse("uint256").unwrap(), span)
            ]
            .into_boxed_slice()
        );
    }

    #[test]
    fn parse_sol_function() {
        let span: Span = SimpleSpan::new(0, 0);

        assert_ok!(
            sol_function(),
            vec![
                Ident("function"),
                Ident("balanceOf"),
                Punct('('),
                Ident("address"),
                Punct(')'),
                Ident("returns"),
                Punct('('),
                Ident("uint256"),
                Punct(')')
            ],
            ast::Definition::SolFunction(ast::SolFunction {
                name: ("balanceOf", span),
                args: Box::new([(DynSolType::parse("address").unwrap(), span)]),
                rets: Box::new([(DynSolType::parse("uint256").unwrap(), span)]),
            })
        );
        assert_ok!(
            sol_function(),
            vec![
                Ident("function"),
                Ident("balanceOf"),
                Punct('('),
                Ident("address"),
                Punct(')'),
                Ident("public"),
                Ident("view"),
                Ident("returns"),
                Punct('('),
                Ident("uint256"),
                Punct(')')
            ],
            ast::Definition::SolFunction(ast::SolFunction {
                name: ("balanceOf", span),
                args: Box::new([(DynSolType::parse("address").unwrap(), span)]),
                rets: Box::new([(DynSolType::parse("uint256").unwrap(), span)]),
            })
        );
    }

    #[test]
    fn parse_sol_event() {
        let span: Span = SimpleSpan::new(0, 0);
        assert_ok!(
            sol_event(),
            vec![
                Ident("event"),
                Ident("Transfer"),
                Punct('('),
                Ident("address"),
                Punct(','),
                Ident("address"),
                Punct(','),
                Ident("uint256"),
                Punct(')')
            ],
            ast::Definition::SolEvent(ast::SolEvent {
                name: ("Transfer", span),
                args: Box::new([
                    (DynSolType::parse("address").unwrap(), span),
                    (DynSolType::parse("address").unwrap(), span),
                    (DynSolType::parse("uint256").unwrap(), span),
                ]),
            })
        );
    }

    #[test]
    fn parse_sol_error() {
        let span: Span = SimpleSpan::new(0, 0);

        assert_ok!(
            sol_error(),
            vec![Ident("error"), Ident("PanicError"), Punct('('), Ident("uint256"), Punct(')')],
            ast::Definition::SolError(ast::SolError {
                name: ("PanicError", span),
                args: Box::new([(DynSolType::parse("uint256").unwrap(), span)]),
            })
        );
    }
}
