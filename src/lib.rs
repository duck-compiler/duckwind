use chumsky::{Parser, error::Rich, extra, prelude::any};

use crate::{
    config_css::{Utility, parse_config},
    lexer::lexer,
    parser::{ParsedUnit, duckwind_parser, make_eoi, make_input},
};

#[cfg(test)]
mod tests;

mod config_css;
mod css_literals;
mod lexer;
mod parser;

pub fn ignore_whitespace<'a>() -> impl Parser<'a, &'a str, (), extra::Err<Rich<'a, char>>> {
    any().filter(|c: &char| *c == ' ').repeated()
}

pub fn ignore_whitespace2<'a>() -> impl Parser<'a, &'a str, (), extra::Err<Rich<'a, char>>> {
    any().filter(|c: &char| c.is_whitespace()).repeated()
}

#[derive(Debug, Clone, Default)]
pub struct CssDef {
    pub media_queries: Vec<String>,
    pub selectors: Vec<String>,
    pub body: String,
}

pub fn is_valid_css_char(c: char) -> bool {
    // https://developer.mozilla.org/en-US/docs/Web/CSS/ident
    return c.is_ascii_digit() || c.is_ascii_alphabetic() || c == '-' || c == '_' || !c.is_ascii();
}

pub fn escape_string_for_css(s: &str) -> String {
    let mut res = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if !is_valid_css_char(c) {
            res.push('\\');
        }
        res.push(c);
    }
    res
}

impl CssDef {
    pub fn to_css(&self) -> String {
        let mut res = String::new();
        let closing_braces = self.media_queries.len() + self.selectors.len();
        for media_query in &self.media_queries {
            res.push_str(&media_query);
            res.push_str("{\n");
        }
        for selector in &self.selectors {
            res.push_str(&selector);
            res.push_str("{\n");
        }
        res.push_str(self.body.as_str());
        res.push('\n');
        for _ in 0..closing_braces {
            res.push_str("}\n");
        }
        res
    }
}

#[derive(Debug, Clone, Default)]
pub struct EmitEnv {
    pub defs: Vec<CssDef>,
    pub utilities: Vec<Utility>,
}

impl EmitEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_config(&mut self, s: &str) {
        let parsed_config = parse_config().parse(s).into_result().expect("parse errors");
        self.utilities.extend(parsed_config.utilities);
    }

    pub fn parse_tailwind_str(&mut self, src: &str) -> Option<CssDef> {
        let leaked = src.to_string().leak() as &'static str;
        let toks = lexer("test", leaked).parse(src).into_output()?;

        let parsed = duckwind_parser(make_input)
            .parse(make_input(make_eoi("test", leaked), toks.as_slice()))
            .into_output()?;

        let mut css_def = CssDef::default();

        let mut pre = parsed.0.utility[..parsed.0.utility.len() - 1]
            .iter()
            .map(|x| {
                let ParsedUnit::String(s) = x.0.clone() else {
                    panic!("can only be string {:?}", x.0)
                };
                s
            })
            .collect::<Vec<_>>();
        let last = parsed.0.utility.last().cloned().unwrap();
        let pre_str = pre.join("-");

        match last.0 {
            ParsedUnit::String(last_str) => {
                pre.push(last_str.clone());
                let full = pre.join("-");
                let class_name = escape_string_for_css(full.as_str());

                println!("{:?}", self.utilities);
                for utility in self.utilities.iter() {
                    if utility.name.as_str() == full.as_str() && !utility.has_value {
                        if let Ok(res) = utility.instantiate(None) {
                            css_def.body = res;
                            return Some(css_def);
                        }
                    }
                }

                for utility in self.utilities.iter() {
                    if utility.name.as_str() == pre_str.as_str() && utility.has_value {
                        if let Ok(res) = utility.instantiate(Some(last_str.as_str())) {
                            css_def.body = res;
                            return Some(css_def);
                        }
                    }
                }
            }
            ParsedUnit::Raw(raw_value) => {
                pre.push(raw_value.clone());
                let full = pre.join("-");
                let class_name = escape_string_for_css(full.as_str());

                for utility in self.utilities.iter() {
                    if utility.name.as_str() == pre_str.as_str() && utility.has_value {
                        if let Ok(res) = utility.instantiate(Some(raw_value.as_str())) {
                            css_def.body = res;
                            return Some(css_def);
                        }
                    }
                }
            }
        }

        None
    }
}
