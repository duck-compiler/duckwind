use chumsky::{
    IterParser, Parser,
    error::Rich,
    extra,
    prelude::{any, choice, just, recursive},
    span::SimpleSpan,
};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Context {
    pub file_name: &'static str,
    pub file_contents: &'static str,
}

pub type DWS = SimpleSpan<usize, Context>;
pub type Spanned<T> = (T, DWS);

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ctrl(char),
    Unit(String),
    Raw(String),
    Whitespace,
}

pub fn token_to_empty_span(t: &mut Spanned<Token>) {
    *t = t.0.empty_span();
}

pub fn empty_span() -> DWS {
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
        (self.clone(), empty_span())
    }
}

pub fn parse_raw_text<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone
{
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

pub fn parse_unit<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    any()
        .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '/' || *c == '#')
        .repeated()
        .at_least(1)
        .collect()
}

pub fn lexer<'a>(
    file_name: &'static str,
    file_contents: &'static str,
) -> impl Parser<'a, &'a str, Vec<Spanned<Token>>, extra::Err<Rich<'a, char>>> + Clone {
    choice((
        parse_raw_text().map(Token::Raw),
        parse_unit().map(Token::Unit),
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

#[cfg(test)]
mod tests {
    use chumsky::Parser;

    use crate::lexer::{Token, empty_span, lexer, token_to_empty_span};

    #[test]
    fn test_parser() {
        let test_cases = vec![
            ("hello", vec![Token::Unit("hello".to_string())]),
            (
                "hover:hello",
                vec![
                    Token::Unit("hover".to_string()),
                    Token::Ctrl(':'),
                    Token::Unit("hello".to_string()),
                ],
            ),
            ("[]", vec![Token::Raw("".to_string())]),
            (
                "bg-[]",
                vec![
                    Token::Unit("bg".to_string()),
                    Token::Ctrl('-'),
                    Token::Raw("".to_string()),
                ],
            ),
            (
                "bg-[red]",
                vec![
                    Token::Unit("bg".to_string()),
                    Token::Ctrl('-'),
                    Token::Raw("red".to_string()),
                ],
            ),
            (
                "bg-[r[e]d]",
                vec![
                    Token::Unit("bg".to_string()),
                    Token::Ctrl('-'),
                    Token::Raw("r[e]d".to_string()),
                ],
            ),
            (
                "hover:bg-[r[e]d]",
                vec![
                    Token::Unit("hover".to_string()),
                    Token::Ctrl(':'),
                    Token::Unit("bg".to_string()),
                    Token::Ctrl('-'),
                    Token::Raw("r[e]d".to_string()),
                ],
            ),
            (
                "p-4",
                vec![
                    Token::Unit("p".to_string()),
                    Token::Ctrl('-'),
                    Token::Unit("4".to_string()),
                ],
            ),
            (
                "p-4 m-8",
                vec![
                    Token::Unit("p".to_string()),
                    Token::Ctrl('-'),
                    Token::Unit("4".to_string()),
                    Token::Whitespace,
                    Token::Unit("m".to_string()),
                    Token::Ctrl('-'),
                    Token::Unit("8".to_string()),
                ],
            ),
            (
                "p-4 \n m-8",
                vec![
                    Token::Unit("p".to_string()),
                    Token::Ctrl('-'),
                    Token::Unit("4".to_string()),
                    Token::Whitespace,
                    Token::Unit("m".to_string()),
                    Token::Ctrl('-'),
                    Token::Unit("8".to_string()),
                ],
            ),
            (
                "p-[100px] m-[3rem]",
                vec![
                    Token::Unit("p".to_string()),
                    Token::Ctrl('-'),
                    Token::Raw("100px".to_string()),
                    Token::Whitespace,
                    Token::Unit("m".to_string()),
                    Token::Ctrl('-'),
                    Token::Raw("3rem".to_string()),
                ],
            ),
        ];

        for (src, expected) in test_cases {
            let parser = lexer("test_file", src);
            let mut result = parser
                .parse(src)
                .into_result()
                .expect(&format!("errors lexing {src}"));
            result.iter_mut().for_each(token_to_empty_span);
            // duckwind_parser(Box::leak(Box::new(Default::default())), make_input)
            //     .parse(make_input(make_eoi("", src), &result));
            assert_eq!(
                expected
                    .iter()
                    .map(|e| (e.clone(), empty_span()))
                    .collect::<Vec<_>>(),
                result,
                "{src} returned {result:?} and not {expected:?}"
            );
        }
    }
}
