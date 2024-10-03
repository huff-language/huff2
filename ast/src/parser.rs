use crate as ast;
use crate::grammar;
use lalrpop_util::{lexer::Token, ParseError};

pub fn parse(src: &str) -> Result<ast::Root, ParseError<usize, Token<'_>, &str>> {
    grammar::RootParser::new().parse(src)
}

#[cfg(test)]
mod tests {
    use crate::HuffDefinition;

    use super::*;
    use alloy_primitives::uint;

    #[test]
    fn constant() {
        assert!(grammar::ConstantParser::new()
            .parse("#define constant TEST =    1")
            .is_ok());
        assert_eq!(
            grammar::ConstantParser::new()
                .parse("#define constant TEST = 1")
                .unwrap(),
            HuffDefinition::Constant {
                name: "TEST",
                value: uint!(1_U256)
            }
        );
    }
}
