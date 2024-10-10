use crate::Spanned;
use chumsky::{
    error::Rich,
    extra,
    primitive::{any, just, one_of},
    text::{self, ascii::keyword},
    IterParser, Parser,
};

/// Lexer token
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'src> {
    Comment(&'src str),
    Keyword(&'src str),
    Ident(&'src str),
    Punct(char),
    Dec(&'src str),
    Hex(&'src str),
    Bin(&'src str),
}

pub fn lexer<'src>(
) -> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char>>> {
    let keyword = just("#")
        .ignore_then(keyword("define").or(keyword("include")))
        .map(Token::Keyword);

    let ident = text::ident().map(Token::Ident);

    let punct = one_of("(){}[]<>:=,").map(Token::Punct);

    let hex = just("0x")
        .ignore_then(text::digits(16))
        .to_slice()
        .map(Token::Hex);

    let bin = just("0b")
        .ignore_then(text::digits(2))
        .to_slice()
        .map(Token::Bin);

    let dec = text::digits(10).to_slice().map(Token::Dec);

    let token = keyword.or(ident).or(punct).or(hex).or(bin).or(dec);

    // comments
    let single_line_comment = just("//")
        .then(any().and_is(just('\n').not()).repeated())
        .padded();
    let multi_line_comment = just("/*")
        .then(any().and_is(just("*/").not()).repeated())
        .then_ignore(just("*/"))
        .padded();
    let comment = single_line_comment.or(multi_line_comment);

    token
        .map_with(|tok, ex| (tok, ex.span()))
        .padded_by(comment.repeated())
        .padded()
        // .recover_with(skip_then_retry_until(any().ignored(), end()))
        .repeated()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::span::SimpleSpan;

    macro_rules! assert_ok {
        ($input:expr, $($expected:tt)*) => {
            assert_eq!(
                lexer().parse($input).into_result(),
                Ok(vec![$($expected)*]),
            );
        };
    }

    // macro_rules! assert_err {
    //     ($input:expr, $expected:expr) => {
    //         assert_eq!(lexer().parse($input).into_result(), Err(vec![$expected]),);
    //     };
    // }

    #[test]
    fn lex_keyword() {
        assert_ok!("#define", (Token::Keyword("define"), SimpleSpan::new(0, 7)));
        assert_ok!(
            "#include",
            (Token::Keyword("include"), SimpleSpan::new(0, 8))
        );
    }

    #[test]
    fn lex_ident() {
        assert_ok!("foo", (Token::Ident("foo"), SimpleSpan::new(0, 3)));
        assert_ok!(
            "foo bar",
            (Token::Ident("foo"), SimpleSpan::new(0, 3)),
            (Token::Ident("bar"), SimpleSpan::new(4, 7))
        );
        // assert_err!(
        //     "foo#define",
        //     Rich::custom(SimpleSpan::new(0, 10), "invalid token")
        // );
    }

    #[test]
    fn lex_punct() {
        assert_ok!("(", (Token::Punct('('), SimpleSpan::new(0, 1)));
        assert_ok!(
            "()",
            (Token::Punct('('), SimpleSpan::new(0, 1)),
            (Token::Punct(')'), SimpleSpan::new(1, 2))
        );
        assert_ok!(
            "{} // comment",
            (Token::Punct('{'), SimpleSpan::new(0, 1)),
            (Token::Punct('}'), SimpleSpan::new(1, 2))
        );
        assert_ok!(
            "{ /* comment */ }",
            (Token::Punct('{'), SimpleSpan::new(0, 1)),
            (Token::Punct('}'), SimpleSpan::new(16, 17))
        );
    }

    #[test]
    fn lex_hex() {
        assert_ok!("0x0", (Token::Hex("0x0"), SimpleSpan::new(0, 3)));
        assert_ok!("0x123", (Token::Hex("0x123"), SimpleSpan::new(0, 5)));
        // assert_err!("0x", SimpleSpan::new(2, 2));
    }

    #[test]
    fn lex_dec() {
        assert_ok!("0", (Token::Dec("0"), SimpleSpan::new(0, 1)));
        assert_ok!("123", (Token::Dec("123"), SimpleSpan::new(0, 3)));
    }

    #[test]
    fn lex_bin() {
        assert_ok!("0b101", (Token::Bin("0b101"), SimpleSpan::new(0, 5)));
        assert_ok!("0b0", (Token::Bin("0b0"), SimpleSpan::new(0, 3)));
    }
}
