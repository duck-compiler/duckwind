use chumsky::Parser;

use crate::{
    escape_string_for_css,
    lexer::{Spanned, Token, empty_span, lexer},
};

fn token_to_empty_span(t: &mut Spanned<Token>) {
    *t = t.0.empty_span();
}

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
    let test_cases = vec![
        ("hello", vec![Token::Ident("hello".to_string())]),
        (
            "hover:hello",
            vec![
                Token::Ident("hover".to_string()),
                Token::Ctrl(':'),
                Token::Ident("hello".to_string()),
            ],
        ),
        ("[]", vec![Token::Raw("".to_string())]),
        (
            "bg-[]",
            vec![
                Token::Ident("bg".to_string()),
                Token::Ctrl('-'),
                Token::Raw("".to_string()),
            ],
        ),
        (
            "bg-[red]",
            vec![
                Token::Ident("bg".to_string()),
                Token::Ctrl('-'),
                Token::Raw("red".to_string()),
            ],
        ),
        (
            "bg-[r[e]d]",
            vec![
                Token::Ident("bg".to_string()),
                Token::Ctrl('-'),
                Token::Raw("r[e]d".to_string()),
            ],
        ),
        (
            "hover:bg-[r[e]d]",
            vec![
                Token::Ident("hover".to_string()),
                Token::Ctrl(':'),
                Token::Ident("bg".to_string()),
                Token::Ctrl('-'),
                Token::Raw("r[e]d".to_string()),
            ],
        ),
        (
            "p-4",
            vec![
                Token::Ident("p".to_string()),
                Token::Ctrl('-'),
                Token::Ident("4".to_string()),
            ],
        ),
        (
            "p-4 m-8",
            vec![
                Token::Ident("p".to_string()),
                Token::Ctrl('-'),
                Token::Ident("4".to_string()),
                Token::Whitespace,
                Token::Ident("m".to_string()),
                Token::Ctrl('-'),
                Token::Ident("8".to_string()),
            ],
        ),
        (
            "p-4 \n m-8",
            vec![
                Token::Ident("p".to_string()),
                Token::Ctrl('-'),
                Token::Ident("4".to_string()),
                Token::Whitespace,
                Token::Ident("m".to_string()),
                Token::Ctrl('-'),
                Token::Ident("8".to_string()),
            ],
        ),
        (
            "p-[100px] m-[3rem]",
            vec![
                Token::Ident("p".to_string()),
                Token::Ctrl('-'),
                Token::Raw("100px".to_string()),
                Token::Whitespace,
                Token::Ident("m".to_string()),
                Token::Ctrl('-'),
                Token::Raw("3rem".to_string()),
            ],
        ),
    ];
    for (src, expected) in test_cases {
        let parser = lexer("test_file", src);
        let mut result = parser
            .parse(src)
            .into_result()
            .expect(&format!("errors lexing {src}"));
        result.iter_mut().for_each(token_to_empty_span);
        assert_eq!(
            expected
                .iter()
                .map(|e| (e.clone(), empty_span()))
                .collect::<Vec<_>>(),
            result,
            "{src} returned {result:?} and not {expected:?}"
        );
    }
}
