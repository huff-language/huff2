use crate::Spanned;
use chumsky::{
    error::Rich,
    extra,
    primitive::{any, choice, just, none_of, one_of},
    text::{self, ascii::keyword},
    IterParser, Parser,
};
use std::fmt;

/// Lex the given source code string into tokens.
pub(crate) fn lex<'a>(src: &'a str) -> Result<Vec<Spanned<Token<'a>>>, Vec<Rich<'a, Token<'a>>>> {
    lexer().parse(src).into_result().map_err(|e| {
        e.into_iter()
            .map(|errs| errs.map_token(Token::Error))
            .collect::<Vec<_>>()
    })
}

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
    String(String),

    Error(char),
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Comment(s)
            | Token::Keyword(s)
            | Token::Ident(s)
            | Token::Dec(s)
            | Token::Hex(s)
            | Token::Bin(s) => write!(f, "{}", s),
            Token::String(s) => write!(f, "{}", s),
            Token::Punct(c) | Token::Error(c) => write!(f, "{}", c),
        }
    }
}

fn lexer<'src>(
) -> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char>>> {
    let validate_end = any()
        .or_not()
        .rewind()
        .validate(|c: Option<char>, e, emitter| {
            if let Some(c) = c {
                if !(c.is_whitespace() || "(){}[]<>:=,/".contains(c)) {
                    emitter.emit(Rich::custom(e.span(), "invalid token"));
                }
            }
        });
    let keyword = just("#")
        .ignore_then(choice((keyword("define"), keyword("include"))))
        .then_ignore(validate_end)
        .map(Token::Keyword);

    let ident = text::ident().then_ignore(validate_end).map(Token::Ident);

    let punct = one_of("(){}[]<>:=,").map(Token::Punct);

    let hex = just("0x")
        .ignore_then(text::digits(16))
        .to_slice()
        .then_ignore(validate_end)
        .map(Token::Hex);

    let bin = just("0b")
        .ignore_then(text::digits(2))
        .then_ignore(validate_end)
        .to_slice()
        .map(Token::Bin);

    let dec = text::digits(10)
        .then_ignore(validate_end)
        .to_slice()
        .map(Token::Dec);

    let string = none_of("\\\"")
        .or(just('\\').ignore_then(just('"')))
        .repeated()
        .to_slice()
        .map(|s: &str| Token::String(s.to_string().replace("\\\"", "\"")))
        .delimited_by(just('"'), just('"'));

    let token = choice((keyword, ident, punct, hex, bin, dec, string));

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

    macro_rules! assert_err {
        ($input:expr, $expected:expr) => {
            assert_eq!(lexer().parse($input).into_result(), Err(vec![$expected]),);
        };
    }

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
        assert_err!(
            "foo#define",
            Rich::custom(SimpleSpan::new(3, 3), "invalid token")
        );
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
        assert_err!("0x0x", Rich::custom(SimpleSpan::new(3, 3), "invalid token"));
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

    #[test]
    fn lex_string() {
        assert_ok!(
            "\"\"",
            (Token::String("".to_string()), SimpleSpan::new(0, 2))
        );
        assert_ok!(
            "\"\\\"\"",
            (Token::String("\"".to_string()), SimpleSpan::new(0, 4))
        );
        assert_ok!(
            "\"foo\"",
            (Token::String("foo".to_string()), SimpleSpan::new(0, 5))
        );
        assert_ok!(
            "\"foo bar\"",
            (Token::String("foo bar".to_string()), SimpleSpan::new(0, 9))
        );
    }
}
