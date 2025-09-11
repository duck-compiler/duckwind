#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct CssDef {
    pub media_queries: Vec<String>,
    pub selector: String,
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
        for media_query in &self.media_queries {
            res.push_str(&media_query);
            res.push('\n');
        }
        res.push_str(&escape_string_for_css(&self.selector));
        res.push_str("{ \n");
        for statement in &self.body {
            res.push_str(statement.as_str());
            res.push(';');
        }
        res.push_str("} \n");
        res
    }
}

#[derive(Debug, Clone)]
pub struct Config {}

impl Default for Config {
    fn default() -> Self {
        Config {}
    }
}

#[derive(Debug, Clone)]
pub struct Parser {
    pub config: Config,
}

impl Parser {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub(crate) fn collect_variants<'a>(&self, s: &'a str) -> (Vec<&'a str>, &'a str) {
        let mut variants: Vec<&'a str> = s.split(":").collect();
        let class = variants
            .pop()
            .expect("split always return at least empty string");
        (variants, class)
    }

    pub fn parse_tailwind_expr(&self, s: &str) -> CssDef {
        let (variants, class) = self.collect_variants(s);
        todo!()
    }

    pub fn look_for_tailwind_classes(&self, s: &str) {}
}
