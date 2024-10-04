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
                error: "the value is too large to fit the target type"
            })
        );
    }

    #[test]
    fn code() {
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
    fn constant() {
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
    fn table() {
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
    fn sol_type_list() {
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
    fn sol_function() {
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
    fn sol_event() {
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
    fn sol_error() {
        assert_eq!(
            grammar::SolErrorParser::new().parse("error PanicError(uint256)"),
            Ok(ast::HuffDefinition::AbiError(ast::AbiError {
                name: "PanicError",
                args: Box::new([DynSolType::parse("uint256").unwrap(),]),
            }))
        );
    }
}
