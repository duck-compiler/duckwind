use chumsky::{
    IterParser, Parser,
    error::Rich,
    extra,
    prelude::{any, choice, just, recursive},
    span::SimpleSpan,
    text::{ascii::ident, whitespace},
};

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct Context {
    pub file_name: &'static str,
    pub file_contents: &'static str,
}

pub(crate) type DWS = SimpleSpan<usize, Context>;
pub(crate) type Spanned<T> = (T, DWS);

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    Ctrl(char),
    Ident(String),
    Raw(String),
    Whitespace,
}

pub(crate) fn empty_span() -> DWS {
    DWS {
        start: 0,
        end: 0,
        context: Context {
            file_name: "",
            file_contents: "",
        },
    }
}

impl Token {
    pub fn empty_span(&self) -> (Self, DWS) {
        (
            self.clone(),
            empty_span(),
        )
    }
}

pub(crate) fn parse_raw_text<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    recursive(|parser| {
        just("[")
            .ignore_then(
                choice((
                    just("[")
                        .rewind()
                        .ignore_then(parser.clone())
                        .map(|x| format!("[{x}]")),
                    any().filter(|c: &char| *c != ']').map(|c| String::from(c)),
                ))
                .repeated()
                .collect::<Vec<String>>(),
            )
            .then_ignore(just("]"))
            .map(|f| f.join(""))
    })
}

pub(crate) fn parse_unit<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    any()
        .filter(|c: &char| c.is_ascii_alphanumeric())
        .repeated()
        .at_least(1)
        .collect()
}

pub(crate) fn lexer<'a>(
    file_name: &'static str,
    file_contents: &'static str,
) -> impl Parser<'a, &'a str, Vec<Spanned<Token>>, extra::Err<Rich<'a, char>>> + Clone {
    choice((
        parse_raw_text().map(Token::Raw),
        parse_unit().map(Token::Ident),
        any()
            .filter(|x| match *x {
                '-' | '*' | '[' | ']' | '(' | ')' | '_' | ':' => true,
                _ => false,
            })
            .map(Token::Ctrl),
        choice((just(" "), just("\n"), just("\t")))
            .repeated()
            .at_least(1)
            .map(|_| Token::Whitespace),
    ))
    .map_with(move |x, e| {
        (
            x,
            DWS {
                start: e.span().start,
                end: e.span().end,
                context: Context {
                    file_name,
                    file_contents,
                },
            },
        )
    })
    .repeated()
    .collect::<Vec<_>>()
}
