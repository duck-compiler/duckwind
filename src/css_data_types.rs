use chumsky::{
    IterParser, Parser,
    error::Rich,
    extra,
    prelude::{any, choice, just, recursive},
};

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Length {
    cap(String),
    ch(String),
    em(String),
    ex(String),
    ic(String),
    lh(String),

    // Root relative
    rcap(String),
    rch(String),
    rem(String),
    rex(String),
    ric(String),
    rlh(String),

    // Viewport relative
    vh(String),
    vw(String),
    vmax(String),
    vmin(String),
    vb(String),
    vi(String),

    // Absolute units
    px(String),
    cm(String),
    mm(String),
    r#Q(String),
    r#in(String),
    pc(String),
    pt(String),

    // Other
    Percentage(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Color {
    Hex(String),
    Named(String),
    Rgb(String),
    Hsl(String),
    Hwb(String),
    Lab(String),
    Lch(String),
    Oklab(String),
    Oklch(String),
    color(String),
    DeviceCmyk(String),
    ColorMix(String),
    ContrastColor(String),
    LightDark(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CssDataType {
    Length(Length),
    Color(Color),
    Ratio(Ratio),
    Number(Number),
    Fr(Fr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ratio(String, String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fr(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Number(String);

pub fn ratio_parser<'a>() -> impl Parser<'a, &'a str, Ratio, extra::Err<Rich<'a, char>>> + Clone {
    any()
        .filter(|c: &char| c.is_ascii_digit() || *c == '.')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .then_ignore(any().filter(|c| *c == ' ').repeated())
        .then_ignore(just("/"))
        .then_ignore(any().filter(|c| *c == ' ').repeated())
        .then(
            any()
                .filter(|c: &char| c.is_ascii_digit() || *c == '.')
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .map(|(a, b)| Ratio(a, b))
}

pub fn fr_parser<'a>() -> impl Parser<'a, &'a str, Fr, extra::Err<Rich<'a, char>>> + Clone {
    any()
        .filter(|c: &char| c.is_ascii_digit() || *c == '.')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .then_ignore(just("fr"))
        .map(Fr)
}

pub fn number_parser<'a>() -> impl Parser<'a, &'a str, Number, extra::Err<Rich<'a, char>>> + Clone {
    any()
        .filter(|c: &char| c.is_ascii_digit() || *c == '.')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(Number)
}

pub fn data_type_parser<'a>()
-> impl Parser<'a, &'a str, CssDataType, extra::Err<Rich<'a, char>>> + Clone {
    choice((
        ratio_parser().map(CssDataType::Ratio),
        fr_parser().map(CssDataType::Fr),
        color_parser().map(CssDataType::Color),
        length_parser().map(CssDataType::Length),
        number_parser().map(CssDataType::Number),
    ))
}

pub fn nested_braces_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    enum R {
        Char(char),
        String(String),
    }
    recursive(|parser| {
        just("(")
            .ignore_then(
                choice((
                    just("(")
                        .rewind()
                        .ignore_then(parser.clone())
                        .map(R::String),
                    any().and_is(just(")").not()).map(R::Char),
                ))
                .repeated()
                .collect::<Vec<_>>(),
            )
            .then_ignore(just(")"))
            .map(|r| {
                let mut s = String::new();
                for r in r {
                    match r {
                        R::String(str) => s.push_str(&str),
                        R::Char(c) => s.push(c),
                    }
                }

                format!("({s})")
            })
    })
}

pub fn color_parser<'a>() -> impl Parser<'a, &'a str, Color, extra::Err<Rich<'a, char>>> + Clone {
    choice((
        just("#")
            .ignore_then(
                any()
                    .filter(|x: &char| x.is_ascii_hexdigit())
                    .repeated()
                    .at_least(6)
                    .at_most(6)
                    .collect::<String>(),
            )
            .map(Color::Hex),
        choice((
            just("black"),
            just("silver"),
            just("gray"),
            just("white"),
            just("maroon"),
            just("red"),
            just("purple"),
            just("fuchsia"),
            just("green"),
            just("lime"),
            just("olive"),
            just("yellow"),
            just("navy"),
            just("blue"),
            just("teal"),
            just("aqua"), //todo(@Apfelfrosch) - more css colors
        ))
        .map(|x| Color::Named(x.to_string())),
        (choice((
            just("rgb"),
            just("hsl"),
            just("hwb"),
            just("lab"),
            just("lch"),
            just("oklab"),
            just("oklch"),
            just("color"),
            just("device-cymk"),
            just("color-mix"),
            just("contrast-color"),
            just("light-dark"),
        ))
        .then(nested_braces_parser())
        .map(|(func, code)| {
            use Color::*;

            let con = match func {
                "rgb" => Rgb,
                "hsl" => Hsl,
                "hwb" => Hwb,
                "lab" => Lab,
                "lch" => Lch,
                "oklab" => Oklab,
                "oklch" => Oklch,
                "color" => color,
                "device-cymk" => DeviceCmyk,
                "color-mix" => ColorMix,
                "contrast-color" => ContrastColor,
                "light-dark" => LightDark,
                _ => panic!("{func:?} not implemented"),
            };

            con(code[1..code.len() - 1].to_string())
        })),
    ))
}

pub fn length_parser<'a>() -> impl Parser<'a, &'a str, Length, extra::Err<Rich<'a, char>>> + Clone {
    any()
        .filter(|c: &char| c.is_ascii_digit() || *c == '.')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .then(choice((
            just("cap"),
            just("ch"),
            just("em"),
            just("ex"),
            just("ic"),
            just("lh"),
            just("rcap"),
            just("rch"),
            just("rem"),
            just("rex"),
            just("ric"),
            just("rlh"),
            just("vh"),
            just("vw"),
            just("vmax"),
            just("vmin"),
            just("vb"),
            just("vi"),
            just("px"),
            just("cm"),
            just("mm"),
            just("Q"),
            just("in"),
            just("pc"),
            just("pt"),
            just("%"),
        )))
        .map(|(a, b)| {
            use Length::*;

            let con = match b {
                "cap" => cap,
                "ch" => ch,
                "em" => em,
                "ex" => ex,
                "ic" => ic,
                "lh" => lh,

                "rcap" => rcap,
                "rch" => rch,
                "rem" => rem,
                "rex" => rex,
                "ric" => ric,
                "rlh" => rlh,

                "vh" => vh,
                "vw" => vw,
                "vmax" => vmax,
                "vmin" => vmin,
                "vb" => vb,
                "vi" => vi,

                "px" => px,
                "cm" => cm,
                "mm" => mm,
                "Q" => r#Q,
                "in" => r#in,
                "pc" => pc,
                "pt" => pt,
                "%" => Percentage,
                _ => panic!("unknown unit {a:?}"),
            };
            con(a)
        })
}

#[cfg(test)]
mod tests {
    use chumsky::Parser;

    use crate::css_data_types::{
        Color, Fr, Number, Ratio, color_parser, data_type_parser, fr_parser, length_parser,
        number_parser, ratio_parser,
    };

    use super::Length;

    #[test]
    fn test_length_parsing() {
        let test_cases = {
            use super::Length::*;

            vec![
                ("1px", px("1".to_string())),
                ("2px", px("2".to_string())),
                ("34.5%", Percentage("34.5".to_string())),
            ]
        };

        for (src, expected) in test_cases {
            let l = length_parser()
                .parse(src)
                .into_result()
                .expect(&format!("length parse error {src:?}"));

            assert_eq!(l, expected, "error: {src:?}");
        }
    }

    #[test]
    fn test_color_parsing() {
        let test_cases = {
            use super::Color::*;

            vec![
                ("red", Named("red".to_string())),
                ("#101010", Hex("101010".to_string())),
                ("rgb(123 200 0)", Rgb("123 200 0".to_string())),
                ("rgb(123 ()200 0)", Rgb("123 ()200 0".to_string())),
                (
                    "color(from origin-color colorspace channel1 channel2 channel3)",
                    color("from origin-color colorspace channel1 channel2 channel3".to_string()),
                ),
                (
                    "light-dark(rgb(0 0 0), rgb(255 255 255))",
                    LightDark("rgb(0 0 0), rgb(255 255 255)".to_string()),
                ),
            ]
        };

        for (src, expected) in test_cases {
            let l = color_parser()
                .parse(src)
                .into_result()
                .expect(&format!("length parse error {src:?}"));

            assert_eq!(l, expected, "error: {src:?}");
        }
    }

    #[test]
    fn test_ratio_parsing() {
        let test_cases = vec![
            ("3/2", Ratio("3".to_string(), "2".to_string())),
            ("3 / 2", Ratio("3".to_string(), "2".to_string())),
            ("3/ 2", Ratio("3".to_string(), "2".to_string())),
            ("3 /2", Ratio("3".to_string(), "2".to_string())),
        ];

        for (src, expected) in test_cases {
            let l = ratio_parser()
                .parse(src)
                .into_result()
                .expect(&format!("length parse error {src:?}"));

            assert_eq!(l, expected, "error: {src:?}");
        }
    }

    #[test]
    fn test_fr_parsing() {
        let test_cases = vec![
            ("3fr", Fr("3".to_string())),
            (".4fr", Fr(".4".to_string())),
            ("3.4fr", Fr("3.4".to_string())),
            ("23.134fr", Fr("23.134".to_string())),
        ];

        for (src, expected) in test_cases {
            let l = fr_parser()
                .parse(src)
                .into_result()
                .expect(&format!("fr parse error {src:?}"));

            assert_eq!(l, expected, "error: {src:?}");
        }
    }

    #[test]
    fn test_number_parsing() {
        let test_cases = vec![
            ("3", Number("3".to_string())),
            ("3.14", Number("3.14".to_string())),
            ("23.134", Number("23.134".to_string())),
        ];

        for (src, expected) in test_cases {
            let l = number_parser()
                .parse(src)
                .into_result()
                .expect(&format!("number parse error {src:?}"));

            assert_eq!(l, expected, "error: {src:?}");
        }
    }

    #[test]
    fn test_full_parser() {
        use super::CssDataType;
        let test_cases = vec![
            ("3", CssDataType::Number(Number("3".to_string()))),
            ("3.14", CssDataType::Number(Number("3.14".to_string()))),
            ("23.134", CssDataType::Number(Number("23.134".to_string()))),
            ("23.134fr", CssDataType::Fr(Fr("23.134".to_string()))),
            ("red", CssDataType::Color(Color::Named("red".to_string()))),
            ("1px", CssDataType::Length(Length::px("1".to_string()))),
            (
                "1/3",
                CssDataType::Ratio(Ratio("1".to_string(), "3".to_string())),
            ),
        ];

        for (src, expected) in test_cases {
            let l = data_type_parser()
                .parse(src)
                .into_result()
                .expect(&format!("full parse error {src:?}"));

            assert_eq!(l, expected, "error: {src:?}");
        }
    }
}
