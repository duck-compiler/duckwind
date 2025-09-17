use chumsky::{
    IterParser, Parser,
    error::Rich,
    extra,
    input::{BorrowInput, Input},
    prelude::{choice, just},
    select_ref,
};

use crate::lexer::{Context, DWS, Spanned, Token};

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedUnit {
    String(String),
    Raw(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub variants: Vec<Vec<Spanned<ParsedUnit>>>,
    pub utility: Vec<Spanned<ParsedUnit>>,
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
    (just(Token::Ctrl('-')).or_not().then(choice((
        select_ref! { Token::Unit(i) => i.to_string() }.map(ParsedUnit::String),
        select_ref! { Token::Raw(i) => i.to_string() }.map(ParsedUnit::Raw),
    ))))
    .map_with(|x, e| {
        (
            match &x.1 {
                ParsedUnit::String(s) => {
                    ParsedUnit::String(format!("{}{s}", x.0.map(|_| "-").unwrap_or_default()))
                }
                ParsedUnit::Raw(s) => {
                    ParsedUnit::Raw(format!("{}{s}", x.0.map(|_| "-").unwrap_or_default()))
                }
            },
            e.span(),
        )
    })
    .separated_by(just(Token::Ctrl('-')))
    .at_least(1)
    .collect::<Vec<_>>()
    .separated_by(just(Token::Ctrl(':')))
    .at_least(1)
    .collect::<Vec<_>>()
    .map(|x| Parsed {
        variants: x[..x.len() - 1]
            .iter()
            .map(|x| {
                // assert!(x.len() == 1, "variant may only consist of 1");
                x.to_owned()
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
