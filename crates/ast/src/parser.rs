use crate as ast;
use crate::grammar;
use lalrpop_util::{lexer::Token, ParseError};
use revm_interpreter::opcode::OpCode;

pub fn parse(src: &str) -> Result<ast::Root, ParseError<usize, Token<'_>, &str>> {
    grammar::RootParser::new().parse(src)
}

// Parses lowercase opcodes.
pub(crate) fn parse_opcode(op: &str) -> Option<OpCode> {
    if op.chars().all(|c| c.is_lowercase()) {
        return OpCode::parse(op.to_uppercase().as_str());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_dyn_abi::DynSolType;
    use alloy_primitives::{uint, U256};
    use revm_interpreter::opcode::OpCode;

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
                error: "the value is too large to fit the target type"
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
                error: "odd number of digits"
            })
        );
    }

    #[test]
    fn macro_parser() {
        assert_eq!(
            grammar::MacroParser::new().parse("macro MAIN() = takes (0) returns (0) { stop }"),
            Ok(ast::HuffDefinition::Macro(ast::Macro {
                name: "MAIN",
                args: Box::new([]),
                takes: 0,
                returns: 0,
                body: Box::new([ast::Instruction::Op(OpCode::STOP)])
            }))
        );
        assert_eq!(
            grammar::MacroParser::new()
                .parse("macro READ_ADDRESS(offset) = takes (0) returns (1) { stop }"),
            Ok(ast::HuffDefinition::Macro(ast::Macro {
                name: "READ_ADDRESS",
                args: Box::new(["offset"]),
                takes: 0,
                returns: 1,
                body: Box::new([ast::Instruction::Op(OpCode::STOP)])
            }))
        );
    }

    #[test]
    fn instruction_parser() {
        assert_eq!(
            grammar::InstructionParser::new().parse("add"),
            Ok(ast::Instruction::Op(OpCode::ADD))
        );
        assert_eq!(
            grammar::InstructionParser::new().parse("0x1"),
            Ok(ast::Instruction::PushAuto(uint!(1_U256)))
        );
        assert_eq!(
            grammar::InstructionParser::new().parse("push32 0x1"),
            Ok(ast::Instruction::Push(OpCode::PUSH32, uint!(1_U256)))
        );
        assert_eq!(
            grammar::InstructionParser::new().parse("x"),
            Ok(ast::Instruction::LabelReference("x"))
        );
        assert_eq!(
            grammar::InstructionParser::new().parse("x:"),
            Ok(ast::Instruction::LabelDefinition("x"))
        );
        assert_eq!(
            grammar::InstructionParser::new().parse("<x>"),
            Ok(ast::Instruction::MacroArgReference("x"))
        );
        assert_eq!(
            grammar::InstructionParser::new().parse("[x]"),
            Ok(ast::Instruction::ConstantReference("x"))
        );
        assert_eq!(
            grammar::InstructionParser::new().parse("__tablestart(TABLE)"),
            Ok(ast::Instruction::Invoke(ast::Invoke::BuiltinTableStart(
                "TABLE"
            )))
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
