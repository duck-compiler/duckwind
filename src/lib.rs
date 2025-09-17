use std::collections::HashMap;

use chumsky::{IterParser, Parser, error::Rich, extra, prelude::any};

use crate::{
    config_css::{Theme, Utility, Variant, config_parser},
    lexer::{DWS, empty_span, lexer},
    parser::{ParsedUnit, duckwind_parser, make_eoi, make_input},
};

#[cfg(test)]
mod tests;

mod config_css;
mod css_literals;
mod lexer;
mod parser;

const DEFAULT_CONFIG: &str = include_str!("css/default_config.css");
const THEME_CONFIG: &str = include_str!("css/theme.css");

pub fn ignore_whitespace<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    any().filter(|c: &char| *c == ' ').repeated().collect()
}

pub fn ignore_whitespace2<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
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
    c.is_ascii_digit() || c.is_ascii_alphabetic() || c == '-' || c == '_' || !c.is_ascii()
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
    pub theme: Theme,
}

impl Default for EmitEnv {
    fn default() -> Self {
        EmitEnv::new_with_default_config()
    }
}

impl EmitEnv {
    pub fn get_breakpoint_var(&self, name: &str) -> Option<String> {
        Some(if let Some(val) = self.theme.vars.get(&format!("breakpoint-{name}")) {
            val.to_string()
        } else {
            match name {
                "sm" => "40rem",
                "md" => "48rem",
                "lg" => "64rem",
                "xl" => "80rem",
                "2xl" => "96rem",
                _ => return None,
            }.to_string()
        })
    }

    pub fn get_container_breakpoint_var(&self, name: &str) -> Option<String> {
        Some(if let Some(val) = self.theme.vars.get(&format!("container-{name}")) {
            val.to_string()
        } else {
            match name {
                "3xs" => "16rem",
                "2xs" => "18rem",
                "xs" => "20rem",
                "sm" => "24rem",
                "md" => "28rem",
                "lg" => "32rem",
                "xl" => "36rem",
                "2xl" => "42rem",
                "3xl" => "48rem",
                "4xl" => "56rem",
                "5xl" => "64rem",
                "6xl" => "72rem",
                "7xl" => "80rem",
                _ => return None,
            }.to_string()
        })
    }

    pub fn resolve_internal_variant(&self, body: &str, v: &[(ParsedUnit, DWS)]) -> Option<String> {
        Some(match &v[0].0 {
            ParsedUnit::String(s) => match s.as_str() {
                "data" => {
                    if let ParsedUnit::Raw(r) = &v[1].0 {
                        format!("&[data-{r}] {{\n{body}\n}}")
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
                        format!("&[data-{joined}] {{\n{body}\n}}")
                    }
                }
                "nth" => match &v[1].0 {
                    ParsedUnit::Raw(raw_value) => {
                        format!("&:nth-child({raw_value}) {{\n{body}\n}}")
                    }
                    _ => return None,
                },
                "nth-last" => match &v[1].0 {
                    ParsedUnit::Raw(raw_value) => {
                        format!("&:nth-last-child({raw_value}) {{\n{body}\n}}")
                    }
                    _ => return None,
                },
                "nth-of-type" => match &v[1].0 {
                    ParsedUnit::Raw(raw_value) => {
                        format!("&:nth-of-type({raw_value}) {{\n{body}\n}}")
                    }
                    _ => return None,
                },
                "nth-last-of-type" => match &v[1].0 {
                    ParsedUnit::Raw(raw_value) => {
                        format!("&:nth-last-of-type({raw_value}) {{\n{body}\n}}")
                    }
                    _ => return None,
                },
                "has" => match &v[1].0 {
                    ParsedUnit::String(_) => {
                        let joined = v[1..]
                            .iter()
                            .map(|x| match &x.0 {
                                ParsedUnit::String(s) => s.to_string(),
                                _ => String::new(),
                            })
                            .collect::<Vec<String>>()
                            .join("-");
                        format!("&:has(:{joined}) {{\n{body}\n}}")
                    }
                    ParsedUnit::Raw(raw_param) => {
                        format!("&:has({raw_param}) {{\n{body}\n}}")
                    }
                },
                "aria" => match &v[1].0 {
                    ParsedUnit::String(next) => match next.as_str() {
                        "busy" => r#"&[aria-busy="true"]"#.to_string(),
                        "checked" => r#"&[aria-checked="true"]"#.to_string(),
                        "disabled" => r#"&[aria-disabled="true"]"#.to_string(),
                        "expanded" => r#"&[aria-expanded="true"]"#.to_string(),
                        "hidden" => r#"&[aria-hidden="true"]"#.to_string(),
                        "pressed" => r#"&[aria-pressed="true"]"#.to_string(),
                        "readonly" => r#"&[aria-readonly="true"]"#.to_string(),
                        "required" => r#"&[aria-required="true"]"#.to_string(),
                        "selected" => r#"&[aria-selected="true"]"#.to_string(),
                        _ => return None,
                    },
                    ParsedUnit::Raw(raw_next) => {
                        format!("&[aria-{raw_next}]")
                    }
                },
                "not" => {
                    let mut other = self.resolve_internal_variant(body, &v[1..])?;
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
                        let joined = v[1..]
                            .iter()
                            .map(|x| match &x.0 {
                                ParsedUnit::String(s) => s.to_string(),
                                _ => String::new(),
                            })
                            .collect::<Vec<String>>()
                            .join("-");
                        if let Some((param, peer_name)) = param_1.split_once("/") {
                            if param == "has" || param == "not" {
                                let mut input =
                                    vec![(ParsedUnit::String(param.to_string()), empty_span())];
                                input.extend_from_slice(&v[2..]);
                                let res = self.resolve_internal_variant(body, input.as_slice())?;
                                let cond = &res[1..res.find("{").unwrap()];
                                format!(
                                    "&:is(:where(.peer{}){cond} ~ *) {{\n{body}\n}}",
                                    escape_string_for_css(&format!("/{peer_name}")),
                                )
                            } else {
                                format!(
                                    "&:is(:where(.peer{}):is(:{joined}) ~ *) {{\n{body}\n}}",
                                    escape_string_for_css(&format!("/{peer_name}"))
                                )
                            }
                        } else if param_1 == "has" || param_1 == "not" {
                            let mut input =
                                vec![(ParsedUnit::String(param_1.to_string()), empty_span())];
                            input.extend_from_slice(&v[2..]);
                            let res = self.resolve_internal_variant(body, input.as_slice())?;
                            let cond = &res[1..res.find("{").unwrap()];
                            format!("&:is(:where(.peer){cond} ~ *) {{\n{body}\n}}",)
                        } else {
                            format!("&:is(:where(.peer):is(:{joined}) ~ *) {{\n{body}\n}}",)
                        }
                    }
                    ParsedUnit::Raw(param_1) => {
                        if param_1.contains("&") {
                            let replaced = param_1.replace("&", ":where(.peer) ~ *");
                            format!("&:is({replaced}) {{\n{body}\n}}")
                        } else {
                            format!("&:is(:where(.peer):is({param_1}) ~ *) {{\n{body}\n}}",)
                        }
                    }
                },
                "in" => match &v[1].0 {
                    ParsedUnit::String(param_1) => {
                        if param_1 == "has" || param_1 == "not" {
                            let mut input =
                                vec![(ParsedUnit::String(param_1.to_string()), empty_span())];
                            input.extend_from_slice(&v[2..]);
                            let res = self.resolve_internal_variant(body, input.as_slice())?;
                            let cond = &res[1..res.find("{").unwrap()];
                            format!("&:is(:where({cond}) *) {{\n{body}\n}}",)
                        } else {
                            let joined = v[1..]
                                .iter()
                                .map(|x| match &x.0 {
                                    ParsedUnit::String(s) => s.to_string(),
                                    _ => String::new(),
                                })
                                .collect::<Vec<String>>()
                                .join("-");
                            format!("&:is(:where(:{joined}) *) {{\n{body}\n}}",)
                        }
                    }
                    ParsedUnit::Raw(param_1) => {
                        format!("&:is(:where({param_1}) *) {{\n{body}\n}}")
                    }
                },
                "group" => match &v[1].0 {
                    ParsedUnit::String(param_1) => {
                        let joined = v[1..]
                            .iter()
                            .map(|x| match &x.0 {
                                ParsedUnit::String(s) => s.to_string(),
                                _ => String::new(),
                            })
                            .collect::<Vec<String>>()
                            .join("-");
                        if let Some((param, group_name)) = param_1.split_once("/") {
                            if param == "has" || param == "not" {
                                let mut input =
                                    vec![(ParsedUnit::String(param.to_string()), empty_span())];
                                input.extend_from_slice(&v[2..]);
                                let res = self.resolve_internal_variant(body, input.as_slice())?;
                                let cond = &res[1..res.find("{").unwrap()];
                                format!(
                                    "&:is(:where(.group{}){cond} *) {{\n{body}\n}}",
                                    escape_string_for_css(&format!("/{group_name}")),
                                )
                            } else {
                                format!(
                                    "&:is(:where(.group{}):is(:{joined}) *) {{\n{body}\n}}",
                                    escape_string_for_css(&format!("/{group_name}")),
                                )
                            }
                        } else if param_1 == "has" || param_1 == "not" {
                            let mut input =
                                vec![(ParsedUnit::String(param_1.to_string()), empty_span())];
                            input.extend_from_slice(&v[2..]);
                            let res = self.resolve_internal_variant(body, input.as_slice())?;
                            let cond = &res[1..res.find("{").unwrap()];
                            format!("&:is(:where(.group){cond} *) {{\n{body}\n}}",)
                        } else {
                            format!("&:is(:where(.group):is(:{joined}) *) {{\n{body}\n}}",)
                        }
                    }
                    ParsedUnit::Raw(param_1) => {
                        if param_1.contains("&") {
                            let replaced = param_1.replace("&", ":where(.group) *");
                            format!("&:is({replaced}) {{\n{body}\n}}")
                        } else {
                            format!("&:is(:where(.group):is({param_1}) *) {{\n{body}\n}}")
                        }
                    }
                },
                _ => return None,
            },
            _ => return None,
        })
    }

    pub fn new_with_default_config() -> Self {
        let mut res = EmitEnv {
            defs: Vec::new(),
            utilities: Vec::new(),
            variants: Vec::new(),
            theme: Theme {
                vars: HashMap::new(),
                keyframes: HashMap::new(),
            },
        };
        res.load_config(DEFAULT_CONFIG);
        res.load_config(THEME_CONFIG);
        res
    }

    pub fn load_config(&mut self, s: &str) {
        let parsed_config = config_parser()
            .parse(s)
            .into_result()
            .expect("parse errors");
        self.utilities.extend(parsed_config.utilities);
        self.variants.extend(parsed_config.variants);
        for theme in parsed_config.themes {
            self.theme.vars.extend(theme.vars);
            self.theme.keyframes.extend(theme.keyframes);
        }
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
                ParsedUnit::String(mut last_str) => {
                    let mut after = None::<String>;
                    if let Some((pre, opacity)) = last_str.split_once("/") {
                        after = Some(opacity.to_string());
                        last_str = pre.to_string();
                    }
                    pre.push(last_str.clone());
                    let full = pre.join("-");

                    for utility in self.utilities.iter() {
                        if utility.name.as_str() == full.as_str()
                            && !utility.has_value
                            && let Ok(res) = utility.instantiate(&self.theme, None, false)
                        {
                            body_to_set = Some(res);
                        }
                    }
                    for utility in self.utilities.iter() {
                        if utility.has_value
                            && full.starts_with(utility.name.as_str())
                            && let Ok(res) = utility.instantiate(
                                &self.theme,
                                Some(&full[&utility.name.len()+1..]),
                                false,
                            )
                        {
                            body_to_set = Some(res);
                        }
                    }

                    for utility in self.utilities.iter() {
                        if utility.name.as_str() == pre_str.as_str()
                            && utility.has_value
                            && let Ok(res) =
                                utility.instantiate(&self.theme, Some(last_str.as_str()), false)
                        {
                            body_to_set = Some(res);
                        }
                    }

                    if let Some(after) = after
                        && let Some(res) = body_to_set.as_mut()
                    {
                        res.push_str(&format!("\nopacity: {after}%;"));
                    }
                }
                ParsedUnit::Raw(raw_value) => {
                    for utility in self.utilities.iter() {
                        if utility.name.as_str() == pre_str.as_str()
                            && utility.has_value
                            && let Ok(res) =
                                utility.instantiate(&self.theme, Some(raw_value.as_str()), true)
                        {
                            body_to_set = Some(res);
                        }
                    }
                }
            }
        }

        css_def.body = body_to_set?;

        for v in parsed.0.variants.iter()
        // .rev()
        {
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
                        match v_str.as_str() {
                            "min" => {
                                if let ParsedUnit::Raw(r) = &v[1].0 {
                                    css_def.body =
                                        format!("@media (width >= {r}) {{\n{}\n}}", css_def.body);
                                }
                            }
                            "max" => {
                                if let ParsedUnit::Raw(r) = &v[1].0 {
                                    css_def.body =
                                        format!("@media (width < {r}) {{\n{}\n}}", css_def.body);
                                }
                            }
                            "@min" => {
                                if let ParsedUnit::Raw(r) = &v[1].0 {
                                    css_def.body = format!(
                                        "@container (width >= {r}) {{\n{}\n}}",
                                        css_def.body
                                    );
                                }
                            }
                            "@max" => {
                                if let ParsedUnit::Raw(r) = &v[1].0 {
                                    css_def.body = format!(
                                        "@container (width < {r}) {{\n{}\n}}",
                                        css_def.body
                                    );
                                }
                            }
                            "supports" => {
                                if let ParsedUnit::Raw(r) = &v[1].0 {
                                    css_def.body =
                                        format!("@supports ({r}) {{\n{}\n}}", css_def.body);
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
                            "not" if v[1].0 == ParsedUnit::String("supports".to_string()) => {
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
                                    css_def.body = format!(
                                        "@supports (not {joined}) {{\n{}\n}}",
                                        css_def.body
                                    );
                                }
                            }
                            _ => {
                                let joined = v[0..]
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
                                if let Some(variant) = self
                                    .variants
                                    .iter()
                                    .find(|x| x.name.as_str() == joined.as_str())
                                {
                                    css_def.body = dbg!(variant.instantiate(&css_def.body));
                                } else {
                                    css_def.body =
                                        self.resolve_internal_variant(css_def.body.as_str(), v)?;
                                }
                            }
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
        }

        Some(css_def)
    }
}
