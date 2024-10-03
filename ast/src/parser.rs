use crate as ast;
use crate::grammar;
use lalrpop_util::{lexer::Token, ParseError};

pub fn parse(src: &str) -> Result<ast::Root, ParseError<usize, Token<'_>, &str>> {
    grammar::RootParser::new().parse(src)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{uint, U256};

    #[test]
    fn word() {
        assert_eq!(grammar::WordParser::new().parse("0x0"), Ok(U256::ZERO));
        assert_eq!(grammar::WordParser::new().parse("0x1"), Ok(uint!(1_U256)));
        assert_eq!(
            grammar::WordParser::new()
                .parse("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
            Ok(U256::MAX)
        );
        assert_eq!(
            grammar::WordParser::new()
                .parse("0x10000000000000000000000000000000000000000000000000000000000000000"),
            Ok(U256::MAX + uint!(1_U256))
        );
    }

    #[test]
    fn constant() {
        let want = Ok(ast::HuffDefinition::Constant {
            name: "TEST",
            value: uint!(1_U256),
        });
        assert_eq!(
            grammar::ConstantParser::new().parse("#define constant TEST = 0x1"),
            want
        );
        assert_eq!(
            grammar::ConstantParser::new().parse(" #define constant TEST = 0x1 "),
            want
        );
        assert_eq!(
            grammar::ConstantParser::new()
                .parse("#define constant TEST /* comment */ = 0x1 // comment"),
            want
        );
    }
}
