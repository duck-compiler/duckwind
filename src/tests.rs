use crate::{EmitEnv, escape_string_for_css};

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
fn test_css_def() {
    let mut emit_env = EmitEnv::default();
    dbg!(&emit_env.variants);

    let test_cases = vec![
        ("text-[3cm]", "a"),
        ("@3xl:peer-not-has-focus-within:bg-red", "a"),
        ("abb-bbb:bg-red", "a"),
        ("starting:bg-red/75", "a"),
        ("group-has/abc-checked:bg-red", "a"),
        ("not-not-group-hover/abc:bg-#000000", "a"),
    ];

    for (src, expected) in test_cases {
        let escaped = emit_env.parse_tailwind_str(src);
        dbg!(escaped);
        assert!(false);
        // assert_eq!(
        //     escaped, expected,
        //     "{src} returned {escaped} and not {expected}"
        // );
    }
}
