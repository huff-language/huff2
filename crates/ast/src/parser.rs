use crate as ast;
use crate::grammar;
use alloy_primitives::U256;
use evm_glue::opcodes::Opcode;
use lalrpop_util::{lexer::Token, ParseError};

pub fn parse(src: &str) -> Result<ast::Root, ParseError<usize, Token<'_>, ast::Error>> {
    grammar::RootParser::new().parse(src)
}

pub(crate) fn u256_as_push_data<'a, const N: usize>(
    value: U256,
) -> Result<[u8; N], ParseError<usize, Token<'a>, ast::Error>> {
    if value.byte_len() > N {
        return Err(ParseError::User {
            error: ast::Error::Todo(format!("word too large for PUSH{}", N)),
        });
    }
    let input = value.to_be_bytes::<32>();
    let mut output = [0u8; N];
    output.copy_from_slice(&input[32 - N..32]);

    Ok(output)
}

pub(crate) fn u256_as_push<'src>(value: U256) -> Opcode {
    match value.byte_len() {
        0..=1 => u256_as_push_data::<1>(value).map(Opcode::PUSH1).unwrap(),
        2 => u256_as_push_data::<2>(value).map(Opcode::PUSH2).unwrap(),
        3 => u256_as_push_data::<3>(value).map(Opcode::PUSH3).unwrap(),
        4 => u256_as_push_data::<4>(value).map(Opcode::PUSH4).unwrap(),
        5 => u256_as_push_data::<5>(value).map(Opcode::PUSH5).unwrap(),
        6 => u256_as_push_data::<6>(value).map(Opcode::PUSH6).unwrap(),
        7 => u256_as_push_data::<7>(value).map(Opcode::PUSH7).unwrap(),
        8 => u256_as_push_data::<8>(value).map(Opcode::PUSH8).unwrap(),
        9 => u256_as_push_data::<9>(value).map(Opcode::PUSH9).unwrap(),
        10 => u256_as_push_data::<10>(value).map(Opcode::PUSH10).unwrap(),
        11 => u256_as_push_data::<11>(value).map(Opcode::PUSH11).unwrap(),
        12 => u256_as_push_data::<12>(value).map(Opcode::PUSH12).unwrap(),
        13 => u256_as_push_data::<13>(value).map(Opcode::PUSH13).unwrap(),
        14 => u256_as_push_data::<14>(value).map(Opcode::PUSH14).unwrap(),
        15 => u256_as_push_data::<15>(value).map(Opcode::PUSH15).unwrap(),
        16 => u256_as_push_data::<16>(value).map(Opcode::PUSH16).unwrap(),
        17 => u256_as_push_data::<17>(value).map(Opcode::PUSH17).unwrap(),
        18 => u256_as_push_data::<18>(value).map(Opcode::PUSH18).unwrap(),
        19 => u256_as_push_data::<19>(value).map(Opcode::PUSH19).unwrap(),
        20 => u256_as_push_data::<20>(value).map(Opcode::PUSH20).unwrap(),
        21 => u256_as_push_data::<21>(value).map(Opcode::PUSH21).unwrap(),
        22 => u256_as_push_data::<22>(value).map(Opcode::PUSH22).unwrap(),
        23 => u256_as_push_data::<23>(value).map(Opcode::PUSH23).unwrap(),
        24 => u256_as_push_data::<24>(value).map(Opcode::PUSH24).unwrap(),
        25 => u256_as_push_data::<25>(value).map(Opcode::PUSH25).unwrap(),
        26 => u256_as_push_data::<26>(value).map(Opcode::PUSH26).unwrap(),
        27 => u256_as_push_data::<27>(value).map(Opcode::PUSH27).unwrap(),
        28 => u256_as_push_data::<28>(value).map(Opcode::PUSH28).unwrap(),
        29 => u256_as_push_data::<29>(value).map(Opcode::PUSH29).unwrap(),
        30 => u256_as_push_data::<30>(value).map(Opcode::PUSH30).unwrap(),
        31 => u256_as_push_data::<31>(value).map(Opcode::PUSH31).unwrap(),
        32 => u256_as_push_data::<32>(value).map(Opcode::PUSH32).unwrap(),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_dyn_abi::DynSolType;
    use alloy_primitives::{hex, ruint, uint, U256};

    #[test]
    fn word_parser() {
        assert_eq!(grammar::WordParser::new().parse("0x0"), Ok(U256::ZERO));
        assert_eq!(grammar::WordParser::new().parse("0x1"), Ok(uint!(1_U256)));
        assert_eq!(grammar::WordParser::new().parse("0b10"), Ok(uint!(2_U256)));
        assert_eq!(
            grammar::WordParser::new()
                .parse("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
            Ok(U256::MAX)
        );
        assert_eq!(
            grammar::WordParser::new()
                .parse("0x10000000000000000000000000000000000000000000000000000000000000000"),
            Err(ParseError::User {
                error: ast::Error::WordOverflow(ruint::ParseError::BaseConvertError(
                    ruint::BaseConvertError::Overflow
                ))
            })
        );
    }

    #[test]
    fn code_parser() {
        assert_eq!(
            grammar::CodeParser::new().parse("0xc0de"),
            Ok(vec![0xc0, 0xde])
        );
        assert_eq!(
            grammar::CodeParser::new().parse("0x0"),
            Err(ParseError::User {
                error: ast::Error::BytesOddLength(hex::FromHexError::OddLength)
            })
        );
    }

    #[test]
    fn macro_parser() {
        assert_eq!(
            grammar::MacroParser::new().parse("macro MAIN() = { }"),
            Ok(ast::HuffDefinition::Macro(ast::Macro {
                name: "MAIN",
                args: Box::new([]),
                takes_returns: None,
                body: Box::new([])
            }))
        );
        assert_eq!(
            grammar::MacroParser::new()
                .parse("macro READ_ADDRESS(offset) = takes (0) returns (1) { stop }"),
            Ok(ast::HuffDefinition::Macro(ast::Macro {
                name: "READ_ADDRESS",
                args: Box::new(["offset"]),
                takes_returns: Some((0, 1)),
                body: Box::new([ast::Instruction::Op(Opcode::STOP)])
            }))
        );
    }

    #[test]
    fn macro_statement_parser() {
        assert_eq!(
            grammar::MacroStatementParser::new().parse("x:"),
            Ok(ast::MacroStatement::LabelDefinition("x"))
        );
        assert_eq!(
            grammar::MacroStatementParser::new().parse("__tablestart(TABLE)"),
            Ok(ast::MacroStatement::Invoke(ast::Invoke::BuiltinTableStart(
                "TABLE"
            )))
        );
        assert_eq!(
            grammar::MacroStatementParser::new().parse("READ_ADDRESS(0x4)"),
            Ok(ast::MacroStatement::Invoke(ast::Invoke::Macro {
                name: "READ_ADDRESS",
                args: Box::new([ast::Instruction::Op(Opcode::PUSH1([0x04]))])
            }))
        );
    }

    #[test]
    fn instruction_parser() {
        assert_eq!(
            grammar::InstructionParser::new().parse("add"),
            Ok(ast::Instruction::Op(Opcode::ADD))
        );
        assert_eq!(
            grammar::InstructionParser::new().parse("0x1"),
            Ok(ast::Instruction::Op(Opcode::PUSH1([0x01])))
        );
        assert_eq!(
            grammar::InstructionParser::new().parse("push2 0x1"),
            Ok(ast::Instruction::Op(Opcode::PUSH2([0x00, 0x01])))
        );
        assert_eq!(
            grammar::InstructionParser::new().parse("x"),
            Ok(ast::Instruction::LabelReference("x"))
        );
        assert_eq!(
            grammar::InstructionParser::new().parse("<x>"),
            Ok(ast::Instruction::MacroArgReference("x"))
        );
        assert_eq!(
            grammar::InstructionParser::new().parse("[x]"),
            Ok(ast::Instruction::ConstantReference("x"))
        );
    }

    #[test]
    fn constant_parser() {
        assert_eq!(
            grammar::ConstantParser::new().parse("constant TEST = 0x1"),
            Ok(ast::HuffDefinition::Constant {
                name: "TEST",
                value: uint!(1_U256),
            })
        );
        assert_eq!(
            grammar::ConstantParser::new()
                .parse(" constant TEST /* comment */ = 0b1101 // comment"),
            Ok(ast::HuffDefinition::Constant {
                name: "TEST",
                value: uint!(13_U256),
            })
        );
    }

    #[test]
    fn table_parser() {
        assert_eq!(
            grammar::TableParser::new().parse("table TEST { 0xc0de }"),
            Ok(ast::HuffDefinition::Codetable {
                name: "TEST",
                data: Box::new([0xc0, 0xde])
            })
        );
        assert_eq!(
            grammar::TableParser::new().parse("table TEST { 0xc0de 0xcc00ddee }"),
            Ok(ast::HuffDefinition::Codetable {
                name: "TEST",
                data: Box::new([0xc0, 0xde, 0xcc, 0x00, 0xdd, 0xee])
            })
        );
    }

    #[test]
    fn sol_type_list_parser() {
        assert_eq!(
            grammar::SolTypeListParser::new().parse("(address, uint256)"),
            Ok(vec![
                DynSolType::parse("address").unwrap(),
                DynSolType::parse("uint256").unwrap()
            ]
            .into_boxed_slice())
        );
        assert_eq!(
            grammar::SolTypeListParser::new().parse("(address[] tokens)"),
            Ok(vec![DynSolType::parse("address[]").unwrap(),].into_boxed_slice())
        );
        assert_eq!(
            grammar::SolTypeListParser::new().parse("(address[3] tokens)"),
            Ok(vec![DynSolType::parse("address[3]").unwrap(),].into_boxed_slice())
        );
        assert_eq!(
            grammar::SolTypeListParser::new().parse("((address, (address to, uint256 amount)[]))"),
            Ok(
                vec![DynSolType::parse("(address,(address,uint256)[])").unwrap(),]
                    .into_boxed_slice()
            )
        );
    }

    #[test]
    fn sol_function_parser() {
        assert_eq!(
            grammar::SolFunctionParser::new()
                .parse("function balanceOf(address) returns (uint256)"),
            Ok(ast::HuffDefinition::AbiFunction(ast::AbiFunction {
                name: "balanceOf",
                args: Box::new([DynSolType::parse("address").unwrap()]),
                rets: Box::new([DynSolType::parse("uint256").unwrap()]),
            }))
        );
    }

    #[test]
    fn sol_event_parser() {
        assert_eq!(
            grammar::SolEventParser::new()
                .parse("event Transfer(address from, address to, uint256 value)"),
            Ok(ast::HuffDefinition::AbiEvent(ast::AbiEvent {
                name: "Transfer",
                args: Box::new([
                    DynSolType::parse("address").unwrap(),
                    DynSolType::parse("address").unwrap(),
                    DynSolType::parse("uint256").unwrap()
                ]),
            }))
        );
    }

    #[test]
    fn sol_error_parser() {
        assert_eq!(
            grammar::SolErrorParser::new().parse("error PanicError(uint256)"),
            Ok(ast::HuffDefinition::AbiError(ast::AbiError {
                name: "PanicError",
                args: Box::new([DynSolType::parse("uint256").unwrap(),]),
            }))
        );
    }
}
