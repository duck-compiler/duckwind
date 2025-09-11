use crate::{Config, Parser, escape_string_for_css};

#[test]
fn test_css_escape() {
    let test_cases = vec![
        ("hello", "hello"),
        ("p-[100px]", r#"p-\[100px\]"#),
        ("hover:bg-red", r#"hover\:bg-red"#),
        ("hover:bg-[red]", r#"hover\:bg-\[red\]"#),
    ];
    for (src, expected) in test_cases {
        let escaped = escape_string_for_css(src);
        assert_eq!(
            expected, &escaped,
            "{src} returned {escaped} and not {expected}"
        );
    }
}

#[test]
fn test_parser() {
    let parser = Parser::new(Config::default());
    let test_cases = vec![
        ("hello", (vec![], "hello")),
        ("hover:hello", (vec!["hover"], "hello")),
        ("md:hover:hello", (vec!["md", "hover"], "hello")),
        ("", (vec![], "")),
    ];
    for (src, expected) in test_cases {
        let variants = parser.collect_variants(src);
        assert_eq!(
            expected, variants,
            "{src} returned {variants:?} and not {expected:?}"
        );
    }
}
