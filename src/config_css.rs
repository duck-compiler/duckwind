use chumsky::{
    IterParser, Parser,
    error::Rich,
    extra,
    prelude::{any, choice, just},
};

use crate::{
    css_literals::{CssLiteral, data_type_parser},
    ignore_whitespace, ignore_whitespace2,
};

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
pub enum UtilityInstantiationError {
    DontNeedValue,
    NeedValue,
    NothingMatched,
}

impl Utility {
    pub fn instantiate(&self, value: Option<&str>) -> Result<String, UtilityInstantiationError> {
        if self.has_value {
            if value.is_none() {
                return Err(UtilityInstantiationError::NeedValue);
            }
        } else {
            if value.is_some() {
                return Err(UtilityInstantiationError::DontNeedValue);
            }

            return Ok(self
                .parts
                .iter()
                .map(|x| match x {
                    ParsedCodePart::String(s) => s.clone(),
                    ParsedCodePart::ValueCall(_) => String::new(),
                })
                .collect::<String>());
        }

        let value = value.expect("should be present at this point");

        let literal = data_type_parser()
            .parse(value)
            .into_result()
            .expect("invalid literal");

        let mut i = 0;
        'outer: while i < self.parts.len() {
            let part = &self.parts[i];
            if let ParsedCodePart::ValueCall(call) = part {
                for p in call.params.iter() {
                    if p.literal_matches(&literal) {
                        let mut res_string = String::new();

                        if i > 0 {
                            // Find start of line
                            let mut i_back = i - 1;

                            loop {
                                let current = &self.parts[i_back];
                                match current {
                                    ParsedCodePart::String(s) => {
                                        let newline = s.rfind('\n');
                                        if let Some(newline) = newline {
                                            if newline + 1 < s.len() {
                                                res_string.insert_str(0, &s[newline + 1..]);
                                            }
                                            break;
                                        } else {
                                            res_string.insert_str(0, &s);
                                        }
                                    }
                                    ParsedCodePart::ValueCall(value_call) => {
                                        if value_call
                                            .params
                                            .iter()
                                            .any(|param| param.literal_matches(&literal))
                                        {
                                            res_string.insert_str(0, value);
                                        } else {
                                            continue 'outer;
                                        }
                                    }
                                }
                                if i_back == 0 {
                                    break;
                                } else {
                                    i_back -= 1;
                                }
                            }
                        }

                        res_string.push_str(value);

                        // Find end of line
                        let mut i_forward = i + 1;

                        while i_forward < self.parts.len() {
                            let current = &self.parts[i_forward];
                            match current {
                                ParsedCodePart::String(s) => {
                                    let newline = s.find('\n');
                                    if let Some(newline) = newline {
                                        if newline != 0 {
                                            res_string.push_str(&s[..newline]);
                                        }
                                        break;
                                    } else {
                                        res_string.push_str(&s);
                                    }
                                }
                                ParsedCodePart::ValueCall(value_call) => {
                                    if value_call
                                        .params
                                        .iter()
                                        .any(|param| param.literal_matches(&literal))
                                    {
                                        res_string.push_str(value);
                                    } else {
                                        continue 'outer;
                                    }
                                }
                            }
                            i_forward += 1;
                        }
                        return Ok(res_string);
                    }
                }
            }
            i += 1;
        }

        return Err(UtilityInstantiationError::NothingMatched);
    }
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
    pub fn css_literal_matches(&self, css_literal: &CssLiteral) -> bool {
        match self {
            ValueType::Length => matches!(css_literal, CssLiteral::Length(..)),
            ValueType::Color => matches!(css_literal, CssLiteral::Color(..)),
            ValueType::Ratio => matches!(css_literal, CssLiteral::Ratio(..)),
            ValueType::Number => matches!(
                css_literal,
                CssLiteral::Number(..) | CssLiteral::Integer(..)
            ),
            ValueType::Fr => matches!(css_literal, CssLiteral::Fr(..)),
            ValueType::Integer => matches!(css_literal, CssLiteral::Integer(..)),
            ValueType::Percentage => matches!(css_literal, CssLiteral::Percentage(..)),
            ValueType::AbsoluteSize => matches!(css_literal, CssLiteral::AbsoluteSize(..)),
            ValueType::Angle => matches!(css_literal, CssLiteral::Angle(..)),
            ValueType::Any => true,
            ValueType::Position => matches!(css_literal, CssLiteral::Position(..)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueUsage {
    Type(ValueType),
    ArbType(ValueType),
    Literal(String),
}

impl ValueUsage {
    pub fn literal_matches(&self, css_literal_src: &CssLiteral) -> bool {
        match self {
            ValueUsage::Type(t) | ValueUsage::ArbType(t) => t.css_literal_matches(css_literal_src),
            ValueUsage::Literal(s) => {
                if let CssLiteral::Other(x) = css_literal_src {
                    x == s
                } else {
                    false
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ValueCall {
    pos: (usize, usize), // start, len
    params: Vec<ValueUsage>,
}

pub fn parse_css_data_type<'a>() -> impl Parser<'a, &'a str, ValueType, extra::Err<Rich<'a, char>>>
{
    choice((
        just("length").map(|_| ValueType::Length),
        just("color").map(|_| ValueType::Color),
        just("ratio").map(|_| ValueType::Ratio),
        just("number").map(|_| ValueType::Number),
        just("fraction").map(|_| ValueType::Fr),
        just("integer").map(|_| ValueType::Integer),
        just("absolute-size").map(|_| ValueType::AbsoluteSize),
        just("angle").map(|_| ValueType::Angle),
        just("any").map(|_| ValueType::Any),
        just("position").map(|_| ValueType::Position),
        just("*").map(|_| ValueType::Any),
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

#[derive(Debug, Clone)]
pub enum ConfigUnit {
    Utility(Utility),
}

// pub fn parse_nested_utility_code<'a>()
// -> impl Parser<'a, &'a str, Utility, extra::Err<Rich<'a, char>>> {
// }

pub fn parse_utility_name<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> {
    any()
        .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '*' || *c == '/')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .separated_by(just("-"))
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|x| x.join("-"))
}

#[derive(Debug, Clone)]
pub struct UserConfig {
    pub utilities: Vec<Utility>,
}

pub fn parse_config<'a>() -> impl Parser<'a, &'a str, UserConfig, extra::Err<Rich<'a, char>>> {
    // ignore_whitespace2()
    //     .ignore_then(choice((utility_parser().map(ConfigUnit::Utility),)))
    //     .then_ignore(ignore_whitespace2())
    //     .repeated()
    //     .collect::<Vec<_>>()
    choice((parse_utility().map(ConfigUnit::Utility),))
        .padded()
        .repeated()
        .collect::<Vec<_>>()
        .map(|v| {
            let mut res = UserConfig {
                utilities: Vec::new(),
            };

            for v in v {
                match v {
                    ConfigUnit::Utility(u) => res.utilities.push(u),
                }
            }

            res
        })
}

pub fn parse_utility<'a>() -> impl Parser<'a, &'a str, Utility, extra::Err<Rich<'a, char>>> {
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
                name: if has_value {
                    name[..name.len() - 2].to_owned()
                } else {
                    name
                },
                parts,
                has_value,
            }
        })
}

#[cfg(test)]
mod tests {
    use chumsky::Parser;

    use crate::config_css::{parse_config, parse_utility};

    #[test]
    fn test_utility_parser() {
        // text-[100]
        let util = parse_utility()
            .parse(
                r#"@utility my-utility {
                    text-size: --value(integer, "lit");
                }"#,
            )
            .into_result()
            .expect("utility error");
        // dbg!(util.instantiate(Some("lit")));
        dbg!(util.instantiate(None));
        assert!(false);
    }

    #[test]
    fn test_parse_config() {
        // text-[100]

        let src = r#"@utility test1 { text-size: 30rem; }

            @utility test2-* {
                text-size: --value(length);
            }
        "#;

        let util = parse_config()
            .parse(src)
            .into_result()
            .expect("config parse error");
        // dbg!(util.instantiate(Some("lit")));
        dbg!(util);
        assert!(false);
    }
}
