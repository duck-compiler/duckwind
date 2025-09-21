use std::collections::HashMap;

use chumsky::{
    IterParser, Parser,
    error::Rich,
    extra,
    input::Input,
    prelude::{any, choice, just, recursive},
};

use crate::{
    SpecialParam,
    css_literals::{CssLiteral, data_type_parser},
    ignore_whitespace, ignore_whitespace2,
};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Variant {
    pub name: String,
    pub body: String,
    pub target: usize,
    pub is_short: bool,
}

impl Variant {
    pub fn instantiate(&self, target: &str) -> String {
        let mut res = self.body.clone();
        res.insert_str(self.target, target);
        res
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub name: String,
    pub default_value: Option<String>,
    pub syntax: Option<String>,
}

impl Property {
    pub fn to_css_def(&self) -> String {
        if let Some(default_value) = self.default_value.as_ref() {
            format!(
                "@property {} {{\nsyntax: \"{}\";\ninitial-value: {};\ninherits: false;\n}}\n",
                self.name,
                self.syntax.as_ref().map(String::as_str).unwrap_or("*"),
                default_value
            )
        } else {
            format!(
                "@property {} {{\nsyntax: \"{}\";\ninherits: false;\n}}\n",
                self.name,
                self.syntax.as_ref().map(String::as_str).unwrap_or("*"),
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Utility {
    pub name: String,
    pub parts: Vec<ParsedCodePart>,
    pub has_value: bool,
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub vars: HashMap<String, String>,
    pub keyframes: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UtilityInstantiationError {
    DontNeedValue,
    NeedValue,
    NothingMatched,
}

impl Utility {
    pub fn instantiate(
        &self,
        theme: &Theme,
        value: Option<&str>,
        special_param: Option<&SpecialParam>,
        is_arb: bool,
    ) -> Result<String, UtilityInstantiationError> {
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

        let value = value.expect("should be present at this point"); // is checked above

        let literal = data_type_parser()
            .parse(value)
            .into_output()
            .unwrap_or(CssLiteral::Other(value.to_string()));

        let mut i = 0;
        'outer: while i < self.parts.len() {
            let part = &self.parts[i];
            if let ParsedCodePart::ValueCall(call) = part {
                for p in call.params.iter() {
                    if let Some(replacement) =
                        p.literal_matches(theme, value, &literal, special_param.clone(), is_arb)
                    {
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
                                            res_string.insert_str(0, s);
                                        }
                                    }
                                    ParsedCodePart::ValueCall(value_call) => {
                                        if let Some(replacement) =
                                            value_call.params.iter().find_map(|param| {
                                                param.literal_matches(
                                                    theme,
                                                    value,
                                                    &literal,
                                                    special_param.clone(),
                                                    is_arb,
                                                )
                                            })
                                        {
                                            res_string.insert_str(
                                                0,
                                                &replacement.unwrap_or_else(|| value.to_string()),
                                            );
                                        } else {
                                            i += 1;
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

                        res_string.push_str(&replacement.unwrap_or_else(|| value.to_string()));

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
                                        res_string.push_str(s);
                                    }
                                }
                                ParsedCodePart::ValueCall(value_call) => {
                                    if let Some(replacement) =
                                        value_call.params.iter().find_map(|param| {
                                            param.literal_matches(
                                                theme,
                                                value,
                                                &literal,
                                                special_param.clone(),
                                                is_arb,
                                            )
                                        })
                                    {
                                        res_string.push_str(
                                            &replacement.unwrap_or_else(|| value.to_string()),
                                        );
                                    } else {
                                        i += 1;
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

        Err(UtilityInstantiationError::NothingMatched)
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
    Var(String, usize),
}

fn expand_3_digit_hex(hex: &str) -> String {
    hex.chars()
        .take(3)
        .fold(String::with_capacity(6), |mut acc, c| {
            acc.push(c);
            acc.push(c);
            acc
        })
}

/// alpha given like so: 100%, 0%, 20%
fn insert_alpha(color: &str, alpha: &str) -> Option<String> {
    let color = color.trim_ascii_start().to_string();
    if color.starts_with("#") {
        let percent_value = alpha[..alpha.len() - 1].parse::<f32>().ok()?;
        let hex_alpha_channel = format!(
            "{:02X}",
            if alpha == "100%" {
                255
            } else if alpha == "0%" {
                0
            } else {
                ((255_f32 / 100_f32) * percent_value)
                    .clamp(0_f32, 255_f32)
                    .round() as i32
            }
        );

        if color.len() == 4 {
            // is 3 digit notation
            return Some(format!(
                "#{}{hex_alpha_channel}",
                expand_3_digit_hex(&color[1..]),
            ));
        } else if color.len() == 7 {
            // is 6 digit notation
            return Some(format!("#{color}{hex_alpha_channel}"));
        }
    } else if ["rgb", "hsl", "lab", "lch", "oklab", "oklch", "color"]
        .into_iter()
        .any(|prefix| color.starts_with(prefix))
    {
        // check if color already contains alpha value
        if !color.contains("/") {
            // since alpha value is the always last argument, we can just insert it at the end
            if let Some(closing_brace) = color.rfind(')') {
                let mut result = color.clone();
                result.insert_str(closing_brace, &format!(" / {alpha}"));
                return Some(result);
            }
        }
    }
    None
}

impl ValueUsage {
    pub fn literal_matches<'a>(
        &self,
        theme: &'a Theme,
        value: &str,
        css_literal_src: &CssLiteral,
        special_param: Option<&SpecialParam>,
        is_arb: bool,
    ) -> Option<Option<String>> {
        match self {
            ValueUsage::Type(t) if !is_arb && t.css_literal_matches(css_literal_src) => {
                if let CssLiteral::Color(..) = css_literal_src
                    && let Some(SpecialParam::Transparency(t)) = special_param
                    && let Some(with_alpha) = {
                        dbg!(&value, &t);
                        dbg!(insert_alpha(value, t.as_str()))
                    }
                {
                    return Some(Some(with_alpha));
                }

                Some(None)
            }
            ValueUsage::ArbType(t) if is_arb && t.css_literal_matches(css_literal_src) => {
                Some(None)
            }
            ValueUsage::Literal(s) => {
                if let CssLiteral::Other(x) = css_literal_src
                    && x == s
                {
                    Some(None)
                } else {
                    None
                }
            }
            ValueUsage::Var(var, to_insert) => {
                let mut to_check = var.clone();
                to_check.insert_str(*to_insert, value);
                if let Some(value) = theme.vars.get(to_check.as_str()) {
                    if let Some(SpecialParam::Transparency(t)) = special_param
                        && let Some(inserted) = insert_alpha(value, t.as_str())
                    {
                        return Some(Some(inserted));
                    }
                    Some(Some(value.to_owned()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueCall {
    params: Vec<ValueUsage>,
}

pub fn parse_css_data_type<'a>()
-> impl Parser<'a, &'a str, ValueType, extra::Err<Rich<'a, char>>> + Clone {
    choice((
        // todo: add size
        just("length").map(|_| ValueType::Length),
        just("percentage").map(|_| ValueType::Percentage),
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

pub fn parse_literal<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    just("\"")
        .ignore_then(
            any()
                .and_is(just("\"").not())
                .repeated()
                .collect::<String>(),
        )
        .then_ignore(just("\""))
}

pub fn parse_value_param<'a>()
-> impl Parser<'a, &'a str, ValueUsage, extra::Err<Rich<'a, char>>> + Clone {
    #[derive(Debug, Clone, PartialEq)]
    enum ParseUnit {
        Char(char),
        Target(usize),
    }
    choice((
        just("--").ignore_then(
            choice((
                just("*").map_with(|_, e| {
                    let x: <&'static str as Input<'static>>::Span = e.span();
                    ParseUnit::Target(x.start)
                }),
                any()
                    .filter(|c: &char| c.is_alphanumeric() || *c == '-')
                    .map(ParseUnit::Char),
            ))
            .repeated()
            .collect::<Vec<_>>()
            .filter(|v| v.iter().any(|elem| matches!(elem, ParseUnit::Target(..))))
            .map_with(|x, e| {
                let span: <&'static str as Input<'static>>::Span = e.span();
                let mut target_idx = None;
                let text = x.iter().fold(String::new(), |mut acc, elem| {
                    match elem {
                        ParseUnit::Char(c) => acc.push(*c),
                        ParseUnit::Target(idx) => target_idx = Some(*idx - span.start),
                    }
                    acc
                });
                ValueUsage::Var(text, target_idx.unwrap())
            }),
        ),
        parse_css_data_type().map(ValueUsage::Type),
        just("[")
            .ignore_then(parse_css_data_type())
            .then_ignore(just("]"))
            .map(ValueUsage::ArbType),
        parse_literal().map(ValueUsage::Literal),
    ))
}

pub fn parse_value_call<'a>()
-> impl Parser<'a, &'a str, ValueCall, extra::Err<Rich<'a, char>>> + Clone {
    just("--value")
        .ignore_then(just("("))
        .ignore_then(
            parse_value_param()
                .separated_by(just(",").then(ignore_whitespace()))
                .collect::<Vec<_>>(),
        )
        .then_ignore(just(")"))
        .map(|x| ValueCall { params: x })
}

#[derive(Debug, Clone)]
enum RawParsedCodePart {
    Char(char),
    ValueCall(ValueCall),
    Property(Property),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedCodePart {
    String(String),
    ValueCall(ValueCall),
}

#[derive(Debug, Clone)]
pub enum ConfigUnit {
    Utility(Utility),
    Variant(Variant),
    Theme(Theme),
}

// pub fn parse_nested_utility_code<'a>()
// -> impl Parser<'a, &'a str, Utility, extra::Err<Rich<'a, char>>> {
// }
//

#[derive(Debug, Clone)]
pub enum VariantParseUnit {
    Target(usize),
    Char(char),
}

pub fn variant_rec_text<'a>()
-> impl Parser<'a, &'a str, Vec<VariantParseUnit>, extra::Err<Rich<'a, char>>> + Clone {
    recursive(|s| {
        just("{")
            .ignore_then(
                choice((
                    just("{").rewind().ignore_then(s.clone()).map(
                        |mut x: Vec<VariantParseUnit>| {
                            x.insert(0, VariantParseUnit::Char('{'));
                            x.push(VariantParseUnit::Char('}'));
                            x
                        },
                    ),
                    just("@slot;").map_with(|_, e| {
                        let span: <&'static str as Input<'static>>::Span = e.span();
                        vec![VariantParseUnit::Target(span.start)]
                    }),
                    any()
                        .and_is(just("}").not())
                        .map(|x| vec![VariantParseUnit::Char(x)]),
                ))
                .repeated()
                .collect::<Vec<_>>(),
            )
            .then_ignore(just("}"))
            .map(|x| x.into_iter().flat_map(Vec::into_iter).collect())
    })
}

pub fn variant_short_rec_text<'a>()
-> impl Parser<'a, &'a str, Vec<VariantParseUnit>, extra::Err<Rich<'a, char>>> + Clone {
    recursive(|s| {
        just("(")
            .ignore_then(
                choice((
                    just("(").rewind().ignore_then(s.clone()).map(
                        |mut x: Vec<VariantParseUnit>| {
                            x.insert(0, VariantParseUnit::Char('('));
                            x.push(VariantParseUnit::Char(')'));
                            x
                        },
                    ),
                    any()
                        .and_is(just(")").not())
                        .map(|x| vec![VariantParseUnit::Char(x)]),
                ))
                .repeated()
                .collect::<Vec<_>>(),
            )
            .then_ignore(just(")"))
            .map(|x| x.into_iter().flat_map(Vec::into_iter).collect())
    })
}

pub fn variant_parser<'a>() -> impl Parser<'a, &'a str, Variant, extra::Err<Rich<'a, char>>> {
    choice((
        just("@custom-variant")
            .then(ignore_whitespace2())
            .then(parse_utility_name())
            .then(ignore_whitespace2())
            .then_ignore(just("{").rewind())
            .then(variant_rec_text())
            .filter(|(_, units)| {
                units
                    .iter()
                    .any(|unit| matches!(unit, VariantParseUnit::Target(..)))
            })
            .map(|((((a, b), name), d), units)| {
                let mut s_buf = String::new();
                let mut target = None;

                for unit in units {
                    match unit {
                        VariantParseUnit::Char(c) => s_buf.push(c),
                        VariantParseUnit::Target(idx) => target = Some(idx),
                    }
                }

                let name_len = name.len();

                Variant {
                    name,
                    body: s_buf,
                    target: target.expect("need target") // is checked above with filter
                        - (a.len() + b.len() + d.len() + 1 + name_len),
                    is_short: false,
                }
            }),
        just("@custom-variant")
            .ignore_then(ignore_whitespace2())
            .ignore_then(parse_utility_name())
            .then_ignore(ignore_whitespace2())
            .then_ignore(just("(").rewind())
            .then(variant_short_rec_text())
            .then_ignore(just(";"))
            .map(|(name, units)| {
                let mut s_buf = String::new();

                for unit in units {
                    match unit {
                        VariantParseUnit::Char(c) => s_buf.push(c),
                        _ => panic!("no here"),
                    }
                }
                let len = s_buf.len();
                s_buf.push_str(" {\n\n}");

                Variant {
                    name,
                    body: s_buf,
                    target: len + 2,
                    is_short: true,
                }
            }),
    ))
}

pub fn parse_var<'a>()
-> impl Parser<'a, &'a str, (String, String), extra::Err<Rich<'a, char>>> + Clone {
    just("--")
        .ignore_then(
            any()
                .filter(|c: &char| c.is_alphanumeric() || *c == '-')
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .then_ignore(ignore_whitespace())
        .then_ignore(just(":"))
        .then_ignore(ignore_whitespace())
        .then(
            any()
                .and_is(just(";").not())
                .filter(|c: &char| *c == ' ' || !c.is_whitespace())
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .then_ignore(just(";"))
}

pub fn keyframes_text_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    recursive(|e| {
        just("{")
            .ignore_then(
                choice((
                    just("{").rewind().ignore_then(e.clone()),
                    any().and_is(just("}").not()).map(String::from),
                ))
                .repeated()
                .collect::<Vec<_>>()
                .map(|v| v.join("")),
            )
            .then_ignore(just("}"))
            .map(|parsed| format!("{{{parsed}}}"))
    })
}

pub fn parse_keyframes<'a>()
-> impl Parser<'a, &'a str, (String, String), extra::Err<Rich<'a, char>>> + Clone {
    just("@keyframes")
        .ignore_then(ignore_whitespace2())
        .ignore_then(parse_utility_name())
        .then_ignore(ignore_whitespace2())
        .then(keyframes_text_parser())
}

pub fn parse_theme<'a>() -> impl Parser<'a, &'a str, Theme, extra::Err<Rich<'a, char>>> + Clone {
    #[derive(Debug, Clone, PartialEq)]
    enum ParseUnit {
        Variable(String, String),
        Keyframes(String, String),
    }
    just("@theme")
        .ignore_then(ignore_whitespace())
        .ignore_then(just("{"))
        .ignore_then(ignore_whitespace2())
        .ignore_then(
            choice((
                parse_var().map(|(var_name, var_value)| ParseUnit::Variable(var_name, var_value)),
                parse_keyframes().map(|(keyframes_name, keyframes_src)| {
                    ParseUnit::Keyframes(keyframes_name, keyframes_src)
                }),
            ))
            .padded()
            .repeated()
            .collect::<Vec<_>>(),
        )
        .then_ignore(ignore_whitespace2())
        .then_ignore(just("}"))
        .then_ignore(ignore_whitespace2())
        .map(|vars| {
            vars.into_iter().fold(
                Theme {
                    vars: HashMap::new(),
                    keyframes: HashMap::new(),
                },
                |mut acc, unit| {
                    match unit {
                        ParseUnit::Variable(var_name, var_value) => {
                            acc.vars.insert(var_name, var_value);
                        }
                        ParseUnit::Keyframes(keyframes_name, keyframes_value) => {
                            acc.keyframes.insert(keyframes_name, keyframes_value);
                        }
                    }
                    acc
                },
            )
        })
}

pub fn parse_utility_name<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    any()
        .filter(|c: &char| {
            c.is_ascii_alphanumeric() || *c == '*' || *c == '/' || *c == '@' || *c == '-'
        })
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
    pub variants: Vec<Variant>,
    pub themes: Vec<Theme>,
}

pub fn config_parser<'a>() -> impl Parser<'a, &'a str, UserConfig, extra::Err<Rich<'a, char>>> {
    choice((
        parse_utility().map_with(|x, e| (ConfigUnit::Utility(x), e.span())),
        variant_parser().map_with(|x, e| (ConfigUnit::Variant(x), e.span())),
        parse_theme().map_with(|x, e| (ConfigUnit::Theme(x), e.span())),
    ))
    .padded()
    .repeated()
    .collect::<Vec<_>>()
    .map(|v| {
        let mut res = UserConfig {
            utilities: Vec::new(),
            variants: Vec::new(),
            themes: Vec::new(),
        };

        for (v, span) in v {
            match v {
                ConfigUnit::Utility(u) => res.utilities.push(u),
                ConfigUnit::Variant(mut v) => {
                    if !v.is_short {
                        v.target -= span.start;
                    }
                    res.variants.push(v);
                }
                ConfigUnit::Theme(v) => res.themes.push(v),
            }
        }

        res
    })
}

pub fn parse_braces_into_string<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    recursive(|s| {
        just("(")
            .ignore_then(
                choice((
                    just("(")
                        .rewind()
                        .ignore_then(s.clone())
                        .map(|x: String| format!("({x})")),
                    any().and_is(just(")").not()).map(|x| String::from(x)),
                ))
                .repeated()
                .collect::<Vec<_>>(),
            )
            .then_ignore(just(")"))
            .map(|v| format!("({})", v.join("")))
    })
}

fn parse_utility_text<'a>()
-> impl Parser<'a, &'a str, Vec<RawParsedCodePart>, extra::Err<Rich<'a, char>>> {
    recursive(|s| {
        choice((
            just("@tw-property")
                .then_ignore(ignore_whitespace2())
                .ignore_then(
                    any()
                        .filter(|c: &char| !c.is_whitespace() && *c != ';')
                        .repeated()
                        .at_least(1)
                        .collect::<String>(),
                )
                .then_ignore(ignore_whitespace2())
                .then(
                    (choice((
                        parse_braces_into_string(),
                        any().map(|c: char| String::from(c)),
                    ))
                    .and_is(just(";").not())
                    .and_is(any().filter(|c: &char| c.is_whitespace()).not()))
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>()
                    .map(|v| v.join(""))
                    .or_not(),
                )
                .then_ignore(ignore_whitespace2())
                .then(
                    any()
                        .filter(|c: &char| !c.is_whitespace() && *c != ';')
                        .repeated()
                        .at_least(1)
                        .collect::<String>()
                        .or_not(),
                )
                .map(|((name, default_value), syntax)| {
                    vec![RawParsedCodePart::Property(Property {
                        name,
                        default_value,
                        syntax,
                    })]
                }),
            just("{")
                .ignore_then(s.clone())
                .map(|mut x: Vec<RawParsedCodePart>| {
                    x.insert(0, RawParsedCodePart::Char('{'));
                    x.push(RawParsedCodePart::Char('}'));
                    x
                }),
            parse_value_call().map(|x| vec![RawParsedCodePart::ValueCall(x)]),
            any()
                .and_is(just("}").not())
                .map(|c| vec![RawParsedCodePart::Char(c)]),
        ))
        .repeated()
        .collect::<Vec<_>>()
        .then_ignore(just("}"))
        .map(|x| x.into_iter().flat_map(Vec::into_iter).collect())
    })
}
pub fn parse_utility<'a>() -> impl Parser<'a, &'a str, Utility, extra::Err<Rich<'a, char>>> {
    just("@utility")
        .ignore_then(ignore_whitespace())
        .ignore_then(parse_utility_name())
        .then_ignore(ignore_whitespace())
        .then_ignore(just("{"))
        .then(parse_utility_text())
        .map(|(name, content)| {
            let mut parts = Vec::new();
            let mut buf = String::new();
            let mut properties = Vec::new();

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
                    RawParsedCodePart::Property(property) => properties.push(property),
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
                properties,
            }
        })
}
