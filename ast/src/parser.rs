use crate as ast;
use crate::grammar;
use lalrpop_util::{lexer::Token, ParseError};

pub fn parse(src: &str) -> Result<ast::Root, ParseError<usize, Token<'_>, &str>> {
    grammar::RootParser::new().parse(src)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_dyn_abi::DynSolType;
    use alloy_primitives::{uint, U256};

    #[test]
    fn word() {
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
                error: "number is too big"
            })
        );
    }

    #[test]
    fn constant() {
        assert_eq!(
            grammar::ConstantParser::new().parse("#define constant TEST = 0x1"),
            Ok(ast::HuffDefinition::Constant {
                name: "TEST",
                value: uint!(1_U256),
            })
        );
        assert_eq!(
            grammar::ConstantParser::new()
                .parse(" #define constant TEST /* comment */ = 0b1101 // comment"),
            Ok(ast::HuffDefinition::Constant {
                name: "TEST",
                value: uint!(13_U256),
            })
        );
    }

    #[test]
    fn table() {
        assert_eq!(
            grammar::TableParser::new().parse("#define table TEST { 0xc0fe }"),
            Ok(ast::HuffDefinition::Codetable {
                name: "TEST",
                data: Box::new([0xc0, 0xfe])
            })
        );
        assert_eq!(
            grammar::TableParser::new().parse("#define table TEST { 0xc0fe 0xd00d }"),
            Ok(ast::HuffDefinition::Codetable {
                name: "TEST",
                data: Box::new([0xc0, 0xfe, 0xd0, 0x0d])
            })
        );
    }

    #[test]
    fn sol_function() {
        assert_eq!(
            grammar::SolFunctionParser::new()
                .parse("#define function balanceOf(address) returns (uint256)"),
            Ok(ast::HuffDefinition::AbiFunction(ast::AbiFunction {
                name: "balanceOf",
                args: Box::new([DynSolType::parse("address").unwrap()]),
            }))
        );
    }

    #[test]
    fn sol_event() {
        assert_eq!(
            grammar::SolEventParser::new()
                .parse("#define event Transfer(address from, address to, uint256 value)"),
            Ok(ast::HuffDefinition::AbiEvent(ast::AbiEvent {
                name: "Transfer",
                args: Box::new([
                    DynSolType::parse("address from").unwrap(),
                    DynSolType::parse("address to").unwrap(),
                    DynSolType::parse("uint256 value").unwrap()
                ]),
            }))
        );
    }
}
