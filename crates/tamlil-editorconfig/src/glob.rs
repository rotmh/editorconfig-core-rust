//! Ref: [spec].
//!
//! [spec]: https://spec.editorconfig.org/#glob-expressions

use std::ops::RangeInclusive;

use chumsky::error::Simple;
use chumsky::prelude::{any, choice, just, recursive};
use chumsky::text::int;
use chumsky::{IterParser, Parser};
use either::Either;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error {
    /// Couldn't parse the input as a glob expression.
    Parse,
    /// A range was found that does not hold `num1 < num2` in `num1..num2`.
    InvalidRange,
}

#[derive(Debug)]
pub(crate) struct Glob {
    tokens: Vec<Token>,

    /// Whether there is a [`Token::PathSeparator`] in the tokens.
    has_separator: bool,
}

impl Glob {
    pub(crate) fn new(source: &str) -> Result<Self, Error> {
        let tokens = glob().parse(source).into_output().ok_or(Error::Parse)?;

        let mut ranges = tokens.iter().filter_map(Token::as_numrange);
        let all_ranges_are_valid = ranges.all(|r| r.start() < r.end());
        if !all_ranges_are_valid {
            return Err(Error::InvalidRange);
        }

        let has_separator = tokens.iter().any(Token::is_separator);

        Ok(Self { tokens, has_separator })
    }

    pub(crate) fn matches<S: AsRef<str>>(path: S) -> bool {
        let path = path.as_ref();
        todo!();
    }
}

type SimpleError<'src> = chumsky::extra::Err<Simple<'src, char>>;

fn glob<'src>() -> impl Parser<'src, &'src str, Vec<Token>, SimpleError<'src>> {
    let star = just("*").to(Token::ZeroOrMoreNonSeparator);
    let double_star = just("**").to(Token::ZeroOrMore);
    let question_mark = just("?").to(Token::Any);
    let path_separator = just("/").to(Token::PathSeperator);

    let literal = any().map(Token::Literal);

    // Order matters.
    choice((
        path_separator,
        escaped(),
        numrange(),
        alternates(),
        charset(),
        double_star,
        star,
        question_mark,
        // Last.
        literal,
    ))
    .repeated()
    .collect()
}

fn escaped<'src>() -> impl Parser<'src, &'src str, Token, SimpleError<'src>> {
    just("\\")
        .ignore_then(any().filter(|ch| ['{', '[', '?', '*'].contains(ch)))
        .map(Token::Escaped)
}

fn charset<'src>() -> impl Parser<'src, &'src str, Token, SimpleError<'src>> {
    let open = just("[");
    let close = just("]");
    let negate = just("!").or_not().map(|n| n.is_some());
    let chars = any().filter(|&ch| ch != ']').repeated().collect();
    let body = negate.then(chars);

    body.delimited_by(open, close)
        .map(|(negated, chars)| Token::Set { negated, chars })
}

fn numrange<'src>() -> impl Parser<'src, &'src str, Token, SimpleError<'src>> {
    let open = just("{");
    let close = just("}");
    let negative = just("-").or_not().map(|n| n.is_some());
    let num_u16 = int(10).try_map(|s: &str, span| {
        s.parse::<u16>().map_err(|_e| Simple::new(None, span))
    });
    let number = negative.then(num_u16).map(|(negative, n)| {
        if negative { -(n as i16) as i32 } else { n as i32 }
    });
    let separator = just("..");
    let body = number.then_ignore(separator).then(number);

    // HACK: The range values are validated in the Glob::new function because
    // failing here on `num2 <= num1` will cause the parser to parse the
    // numrange as an alternates with one item.
    body.delimited_by(open, close).map(|(n1, n2)| Token::NumRange(n1..=n2))
}

fn alternates<'src>() -> impl Parser<'src, &'src str, Token, SimpleError<'src>>
{
    recursive(|alternates| {
        let open = just("{");
        let close = just("}");
        let separator = just(",");

        let string =
            any().filter(|&ch| ch != ',' && ch != '}').repeated().collect();

        choice((alternates.map(Either::Right), string.map(Either::Left)))
            .separated_by(separator)
            .collect()
            .delimited_by(open, close)
            .map(|mut alts: Vec<Either<String, Alternates>>| {
                // An alternate with only one element will match as a literal
                // including the bracets (i.e., `{s1}` is `{s1}` and not `s1`).
                if alts.len() == 1 {
                    let el = alts.remove(0).map_left(|s| format!("{{{s}}}"));
                    alts.insert(0, el);
                }
                alts
            })
            .map(Alternates)
    })
    .map(Token::Alternates)
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum Token {
    /// `*`.
    ZeroOrMoreNonSeparator,

    /// `**`.
    ZeroOrMore,

    /// `?`.
    Any,

    /// `[abcd]` or `[!abcd]`.
    Set {
        negated: bool,
        chars: Vec<char>,
    },

    /// `{s1,s2,s3}`.
    Alternates(Alternates),

    /// `{-5..3}`.
    NumRange(RangeInclusive<i32>),

    Literal(char),

    Escaped(char),

    /// `/`.
    PathSeperator,
}

impl Token {
    const fn as_numrange(&self) -> Option<&RangeInclusive<i32>> {
        match self {
            Token::NumRange(range) => Some(range),
            _ => None,
        }
    }

    const fn is_separator(&self) -> bool {
        matches!(self, Self::PathSeperator)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Alternates(Vec<Either<String, Alternates>>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let tokens = Glob::new("[!abcd]\\*?/**/*.c{,c}").unwrap().tokens;
        assert_eq!(
            tokens,
            [
                Token::Set { negated: true, chars: vec!['a', 'b', 'c', 'd'] },
                Token::Escaped('*'),
                Token::Any,
                Token::PathSeperator,
                Token::ZeroOrMore,
                Token::PathSeperator,
                Token::ZeroOrMoreNonSeparator,
                Token::Literal('.'),
                Token::Literal('c'),
                Token::Alternates(Alternates(vec![
                    Either::Left("".to_string()),
                    Either::Left("c".to_string()),
                ])),
            ]
        );
    }

    #[test]
    fn numrange_and_alternates() {
        let tokens =
            Glob::new("{33..39}andthen{a,b,{{44,},d}}").unwrap().tokens;
        assert_eq!(
            tokens,
            [
                Token::NumRange(33..=39),
                Token::Literal('a'),
                Token::Literal('n'),
                Token::Literal('d'),
                Token::Literal('t'),
                Token::Literal('h'),
                Token::Literal('e'),
                Token::Literal('n'),
                Token::Alternates(Alternates(vec![
                    Either::Left("a".to_string()),
                    Either::Left("b".to_string()),
                    Either::Right(Alternates(vec![
                        Either::Right(Alternates(vec![
                            Either::Left("44".to_string()),
                            Either::Left("".to_string()),
                        ])),
                        Either::Left("d".to_string()),
                    ])),
                ])),
            ]
        );
    }

    #[test]
    fn invalid_numrange() {
        assert_eq!(Glob::new("{-33..-34}").unwrap_err(), Error::InvalidRange);
    }

    #[test]
    fn raw_alternates() {
        let tokens = Glob::new("{s1}").unwrap().tokens;
        assert_eq!(
            tokens,
            [Token::Alternates(Alternates(vec![Either::Left(
                "{s1}".to_string()
            )])),]
        );
    }

    #[test]
    fn alternate_empty_elements() {
        let tokens = Glob::new("{,}").unwrap().tokens;
        assert_eq!(
            tokens,
            [Token::Alternates(Alternates(vec![
                Either::Left("".to_string()),
                Either::Left("".to_string()),
            ]))]
        );
    }

    #[test]
    fn escaped_and_literal_mix() {
        let tokens = Glob::new("a\\*b?").unwrap().tokens;
        assert_eq!(
            tokens,
            [
                Token::Literal('a'),
                Token::Escaped('*'),
                Token::Literal('b'),
                Token::Any,
            ]
        );
    }
}
