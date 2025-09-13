use chumsky::{error::Rich, extra, prelude::any, Parser};

use crate::{
    lexer::lexer,
    parser::{ParsedUnit, duckwind_parser, make_eoi, make_input},
};

#[cfg(test)]
mod tests;

mod lexer;
mod parser;
mod css_data_types;
mod config_css;

pub fn ignore_whitespace<'a>() -> impl Parser<'a, &'a str, (), extra::Err<Rich<'a, char>>> {
    any().filter(|c: &char| c.is_ascii_whitespace()).repeated()
}

#[derive(Debug, Clone, Default)]
pub struct CssDef {
    pub media_queries: Vec<String>,
    pub selectors: Vec<String>,
    pub body: Vec<String>,
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
        for statement in &self.body {
            res.push_str(statement.as_str());
            res.push_str(";\n");
        }
        for _ in 0..closing_braces {
            res.push_str("}\n");
        }
        res
    }
}

#[derive(Debug, Clone, Default)]
pub struct EmitEnv {
    pub defs: Vec<CssDef>,
}

impl EmitEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse_tailwind_str(&mut self, src: &str) {
        let leaked = src.to_string().leak() as &'static str;
        let toks = lexer("test", leaked)
            .parse(src)
            .into_result()
            .expect("lex error");

        let parsed = duckwind_parser(make_input)
            .parse(make_input(make_eoi("test", leaked), toks.as_slice()))
            .into_result()
            .expect("parse error");

        let mut css_def = CssDef::default();

        for (v, _) in &parsed.0.variants {
            match v {
                ParsedUnit::String(variant) => {
                    match variant.as_str() {
                        "hover" => {
                            css_def.selectors.push("&:hover".to_string());
                        }
                        "sm" => {
                            css_def.media_queries.push("width >= 40rem".to_string());
                        }
                        "md" => {
                            css_def.media_queries.push("width >= 48rem".to_string());
                        }
                        "lg" => {
                            css_def.media_queries.push("width >= 64rem".to_string());
                        }
                        "xl" => {
                            css_def.media_queries.push("width >= 80rem".to_string());
                        }
                        "2xl" => {
                            css_def.media_queries.push("width >= 96rem".to_string());
                        }
                        _ => panic!("{variant} not implemented"),
                    };
                }
                ParsedUnit::Raw(raw) => {
                    css_def.selectors.push(raw.to_string());
                }
            }
        }

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

        match last.0 {
            ParsedUnit::String(last_str) => {
                pre.push(last_str.clone());
                let full = pre.join("-");
                css_def
                    .selectors
                    .insert(0, format!(".{}", escape_string_for_css(full.as_str())));
                match full.as_str() {
                    "inline" => {
                        ["display: inline"]
                            .into_iter()
                            .for_each(|stmt| css_def.body.push(stmt.to_string()));
                    }
                    "inline-block" => {
                        ["display: inline-block"]
                            .into_iter()
                            .for_each(|stmt| css_def.body.push(stmt.to_string()));
                    }
                    "hidden" => {
                        ["display: none"]
                            .into_iter()
                            .for_each(|stmt| css_def.body.push(stmt.to_string()));
                    }
                    _ => panic!("{full} not implemented"),
                }
            }
            ParsedUnit::Raw(value) => {
                let prefix = pre.join("-");
                pre.push(value.clone());
                let full = pre.join("-");
                css_def
                    .selectors
                    .insert(0, format!(".{}", escape_string_for_css(full.as_str())));
                match prefix.as_str() {
                    "text" => css_def.body.push(format!("text-size: {value}")),
                    _ => {
                        panic!("{prefix:?} ({full:?}) not implemented")
                    }
                }
            }
        }
        dbg!(css_def.to_css());
    }
}
