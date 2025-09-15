use std::fmt::format;

use chumsky::{IterParser, Parser, error::Rich, extra, prelude::any};

use crate::{
    config_css::{Utility, Variant, config_parser},
    lexer::{DWS, empty_span, lexer},
    parser::{ParsedUnit, duckwind_parser, make_eoi, make_input},
};

#[cfg(test)]
mod tests;

mod config_css;
mod css_literals;
mod lexer;
mod parser;

const DEFAULT_CONFIG: &'static str = include_str!("default_config.css");

pub fn ignore_whitespace<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> {
    any().filter(|c: &char| *c == ' ').repeated().collect()
}

pub fn ignore_whitespace2<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> {
    any()
        .filter(|c: &char| c.is_whitespace())
        .repeated()
        .collect()
}

#[derive(Debug, Clone, Default)]
pub struct CssDef {
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
        res.push_str(&self.class_name);
        for pseudo_elements in &self.pseudo_elements {
            res.push_str(&format!("::{}", pseudo_elements));
        }
        res.push_str("{\n");
        opening_braces += 1;
        res.push_str(self.body.as_str());
        res.push('\n');
        for _ in 0..opening_braces {
            res.push_str("}\n");
        }
        res
    }
}

#[derive(Debug, Clone)]
pub struct EmitEnv {
    pub defs: Vec<CssDef>,
    pub utilities: Vec<Utility>,
    pub variants: Vec<Variant>,
}

impl Default for EmitEnv {
    fn default() -> Self {
        EmitEnv::new_with_default_config()
    }
}

impl EmitEnv {
    pub fn resolve_internal_variant(&self, body: &str, v: &[(ParsedUnit, DWS)]) -> String {
        match &v[0].0 {
            ParsedUnit::String(s) => match s.as_str() {
                "has" => match &v[1].0 {
                    ParsedUnit::String(param) => {
                        format!("&:has(:{param}) {{\n{body}\n}}")
                    }
                    ParsedUnit::Raw(raw_param) => {
                        format!("&:has({raw_param}) {{\n{body}\n}}")
                    }
                },
                "not" => {
                    let mut other = self.resolve_internal_variant(body, &v[1..]);
                    if let Some(mut start) = other.find(|c: char| !c.is_whitespace()) {
                        if &other[start..start + 1] == "&" && start + 1 < other.len() {
                            start += 1;
                        }

                        if let Some(newline_index) = other[start..].find('\n') {
                            other.insert(newline_index, ')');
                            other.insert_str(start, ":not(");
                        }
                    }
                    other
                }
                "peer" => match &v[1].0 {
                    ParsedUnit::String(param_1) => {
                        if let Some((param, peer_name)) = param_1.split_once("/") {
                            if param == "has" || param == "not" {
                                let mut input =
                                    vec![(ParsedUnit::String(param.to_string()), empty_span())];
                                input.extend_from_slice(&v[2..]);
                                let res = self.resolve_internal_variant(body, input.as_slice());
                                let cond = &res[1..res.rfind("{").unwrap()];
                                return format!(
                                    "&:is(:where(.peer{}){cond} ~ *) {{\n{body}\n}}",
                                    escape_string_for_css(&format!("/{peer_name}")),
                                );
                            }

                            return format!(
                                "&:is(:where(.peer{}):is(:{param}) ~ *) {{\n{body}\n}}",
                                escape_string_for_css(&format!("/{peer_name}"))
                            );
                        } else {
                            if param_1 == "has" || param_1 == "not" {
                                let mut input =
                                    vec![(ParsedUnit::String(param_1.to_string()), empty_span())];
                                input.extend_from_slice(&v[2..]);
                                let res = self.resolve_internal_variant(body, input.as_slice());
                                let cond = &res[1..res.rfind("{").unwrap()];
                                return format!("&:is(:where(.peer){cond} ~ *) {{\n{body}\n}}",);
                            }
                            return format!(
                                "&:is(:where(.peer):is(:{param_1}) ~ *) {{\n{body}\n}}",
                            );
                        }
                    }
                    ParsedUnit::Raw(param_1) => {
                        if param_1.contains("&") {
                            let replaced = param_1.replace("&", ":where(.peer) ~ *");
                            return format!("&:is({replaced}) {{\n{body}\n}}");
                        } else {
                            return format!("&:is(:where(.peer):is({param_1}) ~ *) {{\n{body}\n}}",);
                        }
                    }
                },
                "in" => match &v[1].0 {
                    ParsedUnit::String(param_1) => {
                        if param_1 == "has" || param_1 == "not" {
                            let mut input =
                                vec![(ParsedUnit::String(param_1.to_string()), empty_span())];
                            input.extend_from_slice(&v[2..]);
                            let res = self.resolve_internal_variant(body, input.as_slice());
                            let cond = &res[1..res.rfind("{").unwrap()];
                            return format!("&:is(:where({cond}) *) {{\n{body}\n}}",);
                        }
                        return format!("&:is(:where(:{param_1}) *) {{\n{body}\n}}",);
                    }
                    ParsedUnit::Raw(param_1) => {
                        return format!("&:is(:where({param_1}) *) {{\n{body}\n}}");
                    }
                },
                "group" => match &v[1].0 {
                    ParsedUnit::String(param_1) => {
                        if let Some((param, group_name)) = param_1.split_once("/") {
                            if param == "has" || param == "not" {
                                let mut input =
                                    vec![(ParsedUnit::String(param.to_string()), empty_span())];
                                input.extend_from_slice(&v[2..]);
                                let res = self.resolve_internal_variant(body, input.as_slice());
                                let cond = &res[1..res.rfind("{").unwrap()];
                                return format!(
                                    "&:is(:where(.group{}){cond} *) {{\n{body}\n}}",
                                    escape_string_for_css(&format!("/{group_name}")),
                                );
                            }

                            return format!(
                                "&:is(:where(.group{}):is(:{param}) *) {{\n{body}\n}}",
                                escape_string_for_css(&format!("/{group_name}")),
                            );
                        } else {
                            if param_1 == "has" || param_1 == "not" {
                                let mut input =
                                    vec![(ParsedUnit::String(param_1.to_string()), empty_span())];
                                input.extend_from_slice(&v[2..]);
                                let res = self.resolve_internal_variant(body, input.as_slice());
                                let cond = &res[1..res.rfind("{").unwrap()];
                                return format!("&:is(:where(.group){cond} *) {{\n{body}\n}}",);
                            }
                            return format!("&:is(:where(.group):is(:{param_1}) *) {{\n{body}\n}}",);
                        }
                    }
                    ParsedUnit::Raw(param_1) => {
                        if param_1.contains("&") {
                            let replaced = param_1.replace("&", ":where(.group) *");
                            return format!("&:is({replaced}) {{\n{body}\n}}");
                        } else {
                            return format!("&:is(:where(.group):is({param_1}) *) {{\n{body}\n}}");
                        }
                    }
                },
                _ => panic!("invalid built-in variant {v:?}"),
            },
            _ => panic!("invalid built-in variant {v:?}"),
        }
    }

    pub fn new_with_default_config() -> Self {
        let mut res = EmitEnv {
            defs: Vec::new(),
            utilities: Vec::new(),
            variants: Vec::new(),
        };
        res.load_config(DEFAULT_CONFIG);
        res
    }

    pub fn load_config(&mut self, s: &str) {
        let parsed_config = config_parser()
            .parse(s)
            .into_result()
            .expect("parse errors");
        self.utilities.extend(parsed_config.utilities);
        self.variants.extend(parsed_config.variants);
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

        let mut body_to_set = None;

        if parsed.0.utility.len() == 1
            && let Some((ParsedUnit::Raw(raw_css), _)) = parsed.0.utility.first()
        {
            body_to_set = Some(raw_css.to_owned());
        } else {
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
                                body_to_set = Some(res);
                            }
                        }
                    }

                    for utility in self.utilities.iter() {
                        if utility.name.as_str() == pre_str.as_str() && utility.has_value {
                            if let Ok(res) = utility.instantiate(Some(last_str.as_str())) {
                                body_to_set = Some(res);
                            }
                        }
                    }
                }
                ParsedUnit::Raw(raw_value) => {
                    for utility in self.utilities.iter() {
                        if utility.name.as_str() == pre_str.as_str() && utility.has_value {
                            if let Ok(res) = utility.instantiate(Some(raw_value.as_str())) {
                                body_to_set = Some(res);
                            }
                        }
                    }
                }
            }
        }

        css_def.body = body_to_set?;

        for v in parsed.0.variants.iter() {
            if v.len() == 1 {
                match &v[0].0 {
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
                        } else {
                            if let Some(variant) = self
                                .variants
                                .iter()
                                .find(|x| x.name.as_str() == v_str.as_str())
                            {
                                css_def.body = dbg!(variant.instantiate(&css_def.body));
                            }
                        }
                    }
                    ParsedUnit::Raw(raw_str) => {
                        if raw_str.starts_with("::") {
                            css_def.pseudo_elements.push(raw_str[2..].to_string());
                        } else {
                            css_def.body = format!("{raw_str} {{\n{}\n}}", css_def.body);
                        }
                    }
                }
            } else {
                match &v[0].0 {
                    ParsedUnit::String(s) => match s.as_str() {
                        "data" => {
                            if let ParsedUnit::Raw(r) = &v[1].0 {
                                css_def.body = format!("&[data-{r}] {{\n{}\n}}", css_def.body);
                            } else {
                                let joined = v[1..]
                                    .iter()
                                    .map(|x| {
                                        if let ParsedUnit::String(s) = &x.0 {
                                            s.to_owned()
                                        } else {
                                            String::new()
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join("-");
                                css_def.body =
                                    format!("&[data-{joined}] {{\n{}\n}}", css_def.body);
                            }
                        }
                        "supports" => {
                            if let ParsedUnit::Raw(r) = &v[1].0 {
                                css_def.body = format!("@supports ({r}) {{\n{}\n}}", css_def.body);
                            } else {
                                let joined = v[1..]
                                    .iter()
                                    .map(|x| {
                                        if let ParsedUnit::String(s) = &x.0 {
                                            s.to_owned()
                                        } else {
                                            String::new()
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join("-");
                                css_def.body =
                                    format!("@supports ({joined}) {{\n{}\n}}", css_def.body);
                            }
                        }
                        "not" if &v[1].0 == &ParsedUnit::String("supports".to_string()) => {
                            if let ParsedUnit::Raw(r) = &v[1].0 {
                                css_def.body =
                                    format!("@supports (not {r}) {{\n{}\n}}", css_def.body);
                            } else {
                                let joined = v[1..]
                                    .iter()
                                    .map(|x| {
                                        if let ParsedUnit::String(s) = &x.0 {
                                            s.to_owned()
                                        } else {
                                            String::new()
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join("-");
                                css_def.body =
                                    format!("@supports (not {joined}) {{\n{}\n}}", css_def.body);
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
                // is built-in
                css_def.body = self.resolve_internal_variant(css_def.body.as_str(), v);
            }
        }

        Some(css_def)
    }
}
