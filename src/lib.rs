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
    pub pseudo_elements: Vec<String>,
    pub class_name: String,
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
        let mut opening_braces = 0;
        let media_query_part = self
            .media_queries
            .iter()
            .map(|x| format!("({x})"))
            .collect::<Vec<_>>()
            .join(" and ");
        if !media_query_part.is_empty() {
            res.push_str(&media_query_part);
            res.push_str("{\n");
            opening_braces += 1;
        }
        res.push_str(&self.class_name);
        for pseudo_elements in &self.pseudo_elements {
            res.push_str(&format!("::{}", pseudo_elements));
        }
        res.push_str("{\n");
        opening_braces += 1;

        for selector in &self.selectors {
            res.push_str(&selector);
            res.push_str("{\n");
            opening_braces += 1;
        }
        res.push_str(self.body.as_str());
        res.push('\n');
        for _ in 0..opening_braces {
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
        let class_name = escape_string_for_css(src);
        css_def.class_name = class_name;

        for (v, _) in parsed.0.variants.iter() {
            match v {
                ParsedUnit::String(v_str) => {
                    if v_str == "before" {
                        css_def.pseudo_elements.push("before".to_string());
                    } else if v_str == "after" {
                        css_def.pseudo_elements.push("after".to_string());
                    } else if v_str == "placeholder" {
                        css_def.pseudo_elements.push("placeholder".to_string());
                    } else if v_str == "file" {
                        css_def.pseudo_elements.push("file".to_string());
                    } else if v_str == "selection" {
                        css_def.pseudo_elements.push("selection".to_string());
                    } else if v_str == "first-letter" {
                        css_def.pseudo_elements.push("first-letter".to_string());
                    } else if v_str == "first-line" {
                        css_def.pseudo_elements.push("first-line".to_string());
                    } else if v_str == "backdrop" {
                        css_def.pseudo_elements.push("backdrop".to_string());
                    }
                }
                ParsedUnit::Raw(raw_str) => {
                    if raw_str.starts_with("::") {
                        css_def.pseudo_elements.push(raw_str[2..].to_string());
                    } else {
                        css_def.selectors.push(raw_str.to_string());
                    }
                }
            }
        }

        if parsed.0.utility.len() == 1
            && let Some((ParsedUnit::Raw(raw_css), _)) = parsed.0.utility.first()
        {
            css_def.body = raw_css.to_owned();
            return Some(css_def);
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
        let pre_str = pre.join("-");

        match last.0 {
            ParsedUnit::String(last_str) => {
                pre.push(last_str.clone());
                let full = pre.join("-");

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
