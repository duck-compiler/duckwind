use chumsky::{
    IterParser, Parser,
    error::Rich,
    extra,
    input::{BorrowInput, Input},
    prelude::{choice, just},
    select_ref,
};

use crate::lexer::{Context, DWS, Spanned, Token, empty_span};

#[derive(Debug, Clone, Default)]
pub struct Config {}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedUnit {
    String(String),
    Raw(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub variants: Vec<Spanned<ParsedUnit>>,
    pub utility: Vec<Spanned<ParsedUnit>>,
}

impl Parsed {
    pub fn new(variants: Vec<Spanned<ParsedUnit>>, utility: Vec<Spanned<ParsedUnit>>) -> Self {
        Self { variants, utility }
    }

    fn new_e(variants: Vec<ParsedUnit>, utility: Vec<ParsedUnit>) -> Self {
        Self {
            variants: variants.into_iter().map(|f| (f, empty_span())).collect(),
            utility: utility.into_iter().map(|f| (f, empty_span())).collect(),
        }
    }

    fn new_empty<T: ToString>(variants: Vec<T>, utility: Vec<T>) -> Parsed {
        Parsed {
            variants: variants
                .into_iter()
                .map(|x| (ParsedUnit::String(x.to_string()), empty_span()))
                .collect(),
            utility: utility
                .into_iter()
                .map(|x| (ParsedUnit::String(x.to_string()), empty_span()))
                .collect(),
        }
    }

    fn new_empty2<T: ToString>(variants: Vec<T>, utility: Vec<ParsedUnit>) -> Parsed {
        Parsed {
            variants: variants
                .into_iter()
                .map(|x| (ParsedUnit::String(x.to_string()), empty_span()))
                .collect(),
            utility: utility.into_iter().map(|x| (x, empty_span())).collect(),
        }
    }

    fn make_empty(&mut self) {
        for c in self.variants.iter_mut() {
            c.1 = empty_span();
        }
        for c in self.utility.iter_mut() {
            c.1 = empty_span();
        }
    }
}

pub fn make_eoi(file_name: &'static str, file_contents: &'static str) -> DWS {
    DWS {
        start: 0,
        end: file_contents.len(),
        context: Context {
            file_name,
            file_contents,
        },
    }
}

pub fn make_input<'a>(
    eoi: DWS,
    toks: &'a [Spanned<Token>],
) -> impl BorrowInput<'a, Token = Token, Span = DWS> {
    toks.map(eoi, |(t, s)| (t, s))
}

pub fn duckwind_parser<'a, I, M>(
    // c: &'static Config,
    _make_input: M,
) -> impl Parser<'a, I, Spanned<Parsed>, extra::Err<Rich<'a, Token, DWS>>> + Clone + 'a
where
    I: BorrowInput<'a, Token = Token, Span = DWS>,
    M: Fn(DWS, &'a [Spanned<Token>]) -> I + Clone + 'static,
{
    (choice((
        select_ref! { Token::Unit(i) => i.to_string() }.map(ParsedUnit::String),
        select_ref! { Token::Raw(i) => i.to_string() }.map(ParsedUnit::Raw),
    ))
    .map_with(|x, e| (x, e.span()))
    .separated_by(just(Token::Ctrl('-')))
    .at_least(1)
    .collect::<Vec<_>>())
    .separated_by(just(Token::Ctrl(':')))
    .at_least(1)
    .collect::<Vec<_>>()
    .map(|x| Parsed {
        variants: x[..x.len() - 1]
            .iter()
            .map(|x| {
                assert!(x.len() == 1, "variant may only consist of 1");
                x[0].clone()
            })
            .collect(),
        utility: {
            let res = x.last().cloned().unwrap();
            assert!(
                res[..res.len() - 1]
                    .iter()
                    .all(|f| matches!(f.0, ParsedUnit::String(..))),
                "only last may be raw"
            );
            res
        },
    })
    .map_with(|x, e| (x, e.span()))
}

#[cfg(test)]
mod tests {
    use chumsky::Parser;

    use crate::{
        lexer::{empty_span, lexer, token_to_empty_span},
        parser::{Parsed, duckwind_parser, make_eoi, make_input},
    };

    #[test]
    fn test_parser() {
        let test_cases = {
            use crate::parser::ParsedUnit::*;
            vec![
                ("bg-red", Parsed::new_empty(vec![], vec!["bg", "red"])),
                (
                    "hover:bg-red",
                    Parsed::new_empty(vec!["hover"], vec!["bg", "red"]),
                ),
                (
                    "md:hover:bg-blue",
                    Parsed::new_empty(vec!["md", "hover"], vec!["bg", "blue"]),
                ),
                (
                    "md:[&:nth-child(-n+3)]:hover:bg-blue",
                    Parsed::new_e(
                        vec![
                            String("md".to_string()),
                            Raw("&:nth-child(-n+3)".to_string()),
                            String("hover".to_string()),
                        ],
                        vec![String("bg".to_string()), String("blue".to_string())],
                    ),
                ),
                (
                    "md:[&:nth-child(-n+3)]:hover:one-two-[raw]",
                    Parsed::new_e(
                        vec![
                            String("md".to_string()),
                            Raw("&:nth-child(-n+3)".to_string()),
                            String("hover".to_string()),
                        ],
                        vec![
                            String("one".to_string()),
                            String("two".to_string()),
                            Raw("raw".to_string()),
                        ],
                    ),
                ),
            ]
        };

        for (src, expected) in test_cases {
            let parser = lexer("test_file", src);
            let mut result = parser
                .parse(src)
                .into_result()
                .expect(&format!("errors lexing {src}"));
            result.iter_mut().for_each(token_to_empty_span);
            let mut result = duckwind_parser(make_input)
                .parse(make_input(make_eoi("test_file", src), &result))
                .into_result()
                .expect(&format!("errors parsing {src}"));
            result.1 = empty_span();
            result.0.make_empty();
            assert_eq!(
                expected, result.0,
                "{src} returned {result:?} and not {expected:?}"
            );
        }
    }
}
