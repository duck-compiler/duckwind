use chumsky::{
    IterParser, Parser,
    error::Rich,
    extra,
    prelude::{any, choice, just},
};

use crate::{css_literals::CssLiteral, ignore_whitespace};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Variant {
    MediaQuery(String),
    PseudoElement(String),
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Utility {
    pub name: String,
    pub parts: Vec<ParsedCodePart>,
    pub has_value: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueUsingUtility {
    pub locations: Vec<(usize, usize, ValueUsage)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    Length,
    Color,
    Ratio,
    Number,
    Fr,
    Integer,
    Percentage,
    AbsoluteSize,
    Angle,
    Any,
    Position,
}

impl ValueType {
    pub fn css_literal_matches(&self, css_literal: CssLiteral) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueUsage {
    Type(ValueType),
    ArbType(ValueType),
    Literal(String),
}

#[derive(Debug, Clone, PartialEq)]
struct ValueCall {
    pos: (usize, usize), // start, len
    params: Vec<ValueUsage>,
}

pub fn parse_css_data_type<'a>() -> impl Parser<'a, &'a str, ValueType, extra::Err<Rich<'a, char>>>
{
    choice((
        just("color").map(|_| ValueType::Color),
        just("ratio").map(|_| ValueType::Ratio),
        just("number").map(|_| ValueType::Number),
        just("fraction").map(|_| ValueType::Fr),
        just("integer").map(|_| ValueType::Integer),
        just("absolute-size").map(|_| ValueType::AbsoluteSize),
        just("angle").map(|_| ValueType::Angle),
        just("any").map(|_| ValueType::Any),
        just("position").map(|_| ValueType::Position),
    ))
}

pub fn parse_literal<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> {
    just("\"")
        .ignore_then(
            any()
                .and_is(just("\"").not())
                .repeated()
                .collect::<String>(),
        )
        .then_ignore(just("\""))
}

pub fn parse_value_param<'a>() -> impl Parser<'a, &'a str, ValueUsage, extra::Err<Rich<'a, char>>> {
    choice((
        parse_css_data_type().map(ValueUsage::Type),
        just("[")
            .ignore_then(parse_css_data_type())
            .then_ignore(just("]"))
            .map(ValueUsage::ArbType),
        parse_literal().map(ValueUsage::Literal),
    ))
}

pub fn parse_value_call<'a>() -> impl Parser<'a, &'a str, ValueCall, extra::Err<Rich<'a, char>>> {
    just("--value")
        .ignore_then(just("("))
        .ignore_then(
            parse_value_param()
                .separated_by(just(",").then(ignore_whitespace()))
                .collect::<Vec<_>>(),
        )
        .then_ignore(just(")"))
        .map_with(|x, e| ValueCall {
            pos: (e.span().start, e.span().end),
            params: x,
        })
}

#[derive(Debug, Clone)]
enum RawParsedCodePart {
    Char(char),
    ValueCall(ValueCall),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedCodePart {
    String(String),
    ValueCall(ValueCall),
}

// pub fn parse_nested_utility_code<'a>()
// -> impl Parser<'a, &'a str, Utility, extra::Err<Rich<'a, char>>> {
// }

pub fn parse_utility_name<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> {
    any()
        .filter(|c: &char| c.is_ascii_alphabetic() || *c == '*')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .separated_by(just("-"))
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|x| x.join("-"))
}

pub fn utility_parser<'a>() -> impl Parser<'a, &'a str, Utility, extra::Err<Rich<'a, char>>> {
    just("@utility")
        .ignore_then(ignore_whitespace())
        .ignore_then(parse_utility_name())
        .then_ignore(ignore_whitespace())
        .then_ignore(just("{"))
        .then_ignore(ignore_whitespace())
        .then(
            choice((
                parse_value_call().map(RawParsedCodePart::ValueCall),
                any()
                    .and_is(just("--value").not())
                    .and_is(just("}").not())
                    .map(RawParsedCodePart::Char),
            ))
            .repeated()
            .collect::<Vec<_>>(),
        )
        .then_ignore(ignore_whitespace())
        .then_ignore(just("}"))
        .map(|(name, content)| {
            let mut parts = Vec::new();
            let mut buf = String::new();

            for c in content {
                match c {
                    RawParsedCodePart::Char(c) => buf.push(c),
                    RawParsedCodePart::ValueCall(e) => {
                        if !buf.is_empty() {
                            parts.push(ParsedCodePart::String(buf.clone()));
                            buf.clear();
                        }
                        parts.push(ParsedCodePart::ValueCall(e));
                    }
                }
            }

            if !buf.is_empty() {
                parts.push(ParsedCodePart::String(buf));
            }

            let has_value = name.ends_with("-*");

            Utility {
                name: name[..name.len() - 2].to_owned(),
                parts,
                has_value,
            }
        })
}

#[cfg(test)]
mod tests {
    use chumsky::Parser;

    use crate::config_css::utility_parser;

    #[test]
    fn test_utility_parser() {
        // text-[100]
        let util = utility_parser()
            .parse(
                r#"@utility my-utility-* {
                    text-size: --value(integer);
                }"#,
            )
            .into_result()
            .expect("utility error");
        dbg!(util);
        assert!(false);
    }
}
