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
}

impl Length {
    pub fn to_css(self) -> String {
        use Length::*;
        match self {
            cap(s) => format!("{s}cap"),
            ch(s) => format!("{s}ch"),
            em(s) => format!("{s}em"),
            ex(s) => format!("{s}ex"),
            ic(s) => format!("{s}ic"),
            lh(s) => format!("{s}lh"),

            rcap(s) => format!("{s}rcap"),
            rch(s) => format!("{s}rch"),
            rem(s) => format!("{s}rem"),
            rex(s) => format!("{s}rex"),
            ric(s) => format!("{s}ric"),
            rlh(s) => format!("{s}rlh"),

            vh(s) => format!("{s}vh"),
            vw(s) => format!("{s}vw"),
            vmax(s) => format!("{s}vmax"),
            vmin(s) => format!("{s}vmin"),
            vb(s) => format!("{s}vb"),
            vi(s) => format!("{s}vi"),

            px(s) => format!("{s}px"),
            cm(s) => format!("{s}cm"),
            mm(s) => format!("{s}mm"),
            r#Q(s) => format!("{s}Q"),
            r#in(s) => format!("{s}in"),
            pc(s) => format!("{s}pc"),
            pt(s) => format!("{s}pt"),
        }
    }
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
pub enum AbsoluteSize {
    XxSmall,
    XSmall,
    Small,
    Medium,
    Large,
    XLarge,
    XxLarge,
    XxxLarge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Angle {
    Deg(String),
    Grad(String),
    Rad(String),
    Turn(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CssDataType {
    Length(Length),
    Color(Color),
    Ratio(String, String),
    Number(String),
    Fr(String),
    Integer(String),
    Percentage(String),
    AbsoluteSize(AbsoluteSize),
    Angle(Angle),
    Any,
    Position(String),
}

#[derive(Debug, Clone)]
pub enum Position {
    Left,
    Right,
    Top,
    Bottom,
    Center,
    Length(Length),
    Percentage(String),
}

pub fn single_position_parser<'a>()
-> impl Parser<'a, &'a str, Position, extra::Err<Rich<'a, char>>> + Clone {
    choice((
        just("left").map(|_| Position::Left),
        just("right").map(|_| Position::Right),
        just("top").map(|_| Position::Top),
        just("bottom").map(|_| Position::Bottom),
        just("center").map(|_| Position::Center),
        length_parser().map(Position::Length),
        percentage_parser().map(Position::Percentage),
    ))
}

pub fn position_parser<'a>()
-> impl Parser<'a, &'a str, Vec<Position>, extra::Err<Rich<'a, char>>> + Clone {
    // https://developer.mozilla.org/en-US/docs/Web/CSS/position_value
    choice((
        single_position_parser()
            .then_ignore(
                any()
                    .filter(|c: &char| c.is_ascii_whitespace())
                    .repeated()
                    .at_least(1),
            )
            .then(single_position_parser())
            .then_ignore(
                any()
                    .filter(|c: &char| c.is_ascii_whitespace())
                    .repeated()
                    .at_least(1),
            )
            .then(single_position_parser())
            .then_ignore(
                any()
                    .filter(|c: &char| c.is_ascii_whitespace())
                    .repeated()
                    .at_least(1),
            )
            .then(single_position_parser())
            .filter(|(((a, b), c), d)| {
                (matches!(a, Position::Left | Position::Right)
                    && matches!(b, Position::Percentage(..) | Position::Length(..))
                    && matches!(c, Position::Top | Position::Bottom)
                    && matches!(d, Position::Percentage(..) | Position::Length(..)))
                    || (matches!(c, Position::Left | Position::Right)
                        && matches!(b, Position::Percentage(..) | Position::Length(..))
                        && matches!(a, Position::Top | Position::Bottom)
                        && matches!(d, Position::Percentage(..) | Position::Length(..)))
            })
            .map(|(((a, b), c), d)| vec![a, b, c, d]),
        single_position_parser()
            .then_ignore(
                any()
                    .filter(|c: &char| c.is_ascii_whitespace())
                    .repeated()
                    .at_least(1),
            )
            .then(single_position_parser())
            .filter(|(a, b)| {
                (matches!(a, Position::Left | Position::Center | Position::Right)
                    && matches!(b, Position::Top | Position::Center | Position::Bottom))
                    || (matches!(b, Position::Left | Position::Center | Position::Right)
                        && matches!(a, Position::Top | Position::Center | Position::Bottom))
            })
            .map(|(a, b)| vec![a, b]),
        single_position_parser()
            .then_ignore(
                any()
                    .filter(|c: &char| c.is_ascii_whitespace())
                    .repeated()
                    .at_least(1),
            )
            .then(single_position_parser())
            .filter(|(a, b)| {
                matches!(
                    a,
                    Position::Left
                        | Position::Center
                        | Position::Right
                        | Position::Percentage(..)
                        | Position::Length(..)
                ) && matches!(
                    b,
                    Position::Top
                        | Position::Center
                        | Position::Bottom
                        | Position::Percentage(..)
                        | Position::Length(..)
                )
            })
            .map(|(a, b)| vec![a, b]),
        single_position_parser().map(|x| vec![x]),
    ))
}

pub fn ratio_parser<'a>()
-> impl Parser<'a, &'a str, (String, String), extra::Err<Rich<'a, char>>> + Clone {
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
}

pub fn fr_parser<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    any()
        .filter(|c: &char| c.is_ascii_digit() || *c == '.')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .then_ignore(just("fr"))
}

pub fn integer_parser<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone
{
    any()
        .filter(|c: &char| c.is_ascii_digit() || *c == '-')
        .repeated()
        .at_least(1)
        .collect::<String>()
}

pub fn percentage_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    any()
        .filter(|c: &char| c.is_ascii_digit() || *c == '.' || *c == '-')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .then_ignore(just("%"))
}

pub fn number_parser<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    any()
        .filter(|c: &char| c.is_ascii_digit() || *c == '-' || *c == '.')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .filter(|s| s.contains('.'))
}

pub fn angle_parser<'a>() -> impl Parser<'a, &'a str, Angle, extra::Err<Rich<'a, char>>> + Clone {
    any()
        .filter(|c: &char| c.is_ascii_digit() || *c == '.')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .then(choice((
            just("deg"),
            just("grad"),
            just("rad"),
            just("turn"),
        )))
        .map(|(s, unit)| {
            use Angle::*;

            let con = match unit {
                "deg" => Deg,
                "grad" => Grad,
                "rad" => Rad,
                "turn" => Turn,
                _ => panic!("angle {unit:?} not implemented"),
            };
            con(s)
        })
}

pub fn absolute_size_parser<'a>()
-> impl Parser<'a, &'a str, AbsoluteSize, extra::Err<Rich<'a, char>>> + Clone {
    choice((
        just("xx-small"),
        just("x-small"),
        just("small"),
        just("medium"),
        just("large"),
        just("x-large"),
        just("xx-large"),
        just("xxx-large"),
    ))
    .map(|size| {
        use AbsoluteSize::*;
        let value = match size {
            "xx-small" => XxSmall,
            "x-small" => XSmall,
            "small" => Small,
            "medium" => Medium,
            "large" => Large,
            "x-large" => XLarge,
            "xx-large" => XxLarge,
            "xxx-large" => XxxLarge,
            _ => panic!("{size:?} not implemented"),
        };

        value
    })
}

pub fn data_type_parser<'a>()
-> impl Parser<'a, &'a str, CssDataType, extra::Err<Rich<'a, char>>> + Clone {
    choice((
        position_parser().map(|x| {
            if x.len() == 1
                && let Some(Position::Length(l)) = x.first()
            {
                return CssDataType::Length(l.clone());
            }

            if x.len() == 1
                && let Some(Position::Percentage(p)) = x.first()
            {
                return CssDataType::Percentage(p.clone());
            }

            CssDataType::Position(
                x.into_iter()
                    .map(|x| match x {
                        Position::Top => "top".to_string(),
                        Position::Bottom => "bottom".to_string(),
                        Position::Left => "left".to_string(),
                        Position::Right => "right".to_string(),
                        Position::Center => "center".to_string(),
                        Position::Percentage(p) => format!("{p}%"),
                        Position::Length(l) => l.to_css(),
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        }),
        ratio_parser().map(|(a, b)| CssDataType::Ratio(a, b)),
        fr_parser().map(CssDataType::Fr),
        color_parser().map(CssDataType::Color),
        angle_parser().map(CssDataType::Angle),
        length_parser().map(CssDataType::Length),
        number_parser().map(CssDataType::Number),
        percentage_parser().map(CssDataType::Percentage),
        integer_parser().map(CssDataType::Integer),
        absolute_size_parser().map(CssDataType::AbsoluteSize),
        just("*").map(|_| CssDataType::Any),
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
        .filter(|c: &char| c.is_ascii_digit() || *c == '.' || *c == '-')
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
                _ => panic!("unknown unit {a:?}"),
            };
            con(a)
        })
}

#[cfg(test)]
mod tests {
    use chumsky::Parser;

    use crate::css_data_types::{
        AbsoluteSize, Angle, Color, color_parser, data_type_parser, fr_parser, length_parser,
        number_parser, ratio_parser,
    };

    use super::Length;

    #[test]
    fn test_length_parsing() {
        let test_cases = {
            use super::Length::*;

            vec![("1px", px("1".to_string())), ("2px", px("2".to_string()))]
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
            ("3/2", ("3".to_string(), "2".to_string())),
            ("3 / 2", ("3".to_string(), "2".to_string())),
            ("3/ 2", ("3".to_string(), "2".to_string())),
            ("3 /2", ("3".to_string(), "2".to_string())),
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
        let test_cases = vec!["3fr", ".4fr", "3.4fr", "23.134fr"];

        for src in test_cases {
            let l = fr_parser()
                .parse(src)
                .into_result()
                .expect(&format!("fr parse error {src:?}"));

            assert_eq!(l, src.replace("fr", ""), "error: {src:?}");
        }
    }

    #[test]
    fn test_number_parsing() {
        let test_cases = vec!["3.14", "23.134"];

        for src in test_cases {
            let l = number_parser()
                .parse(src)
                .into_result()
                .expect(&format!("number parse error {src:?}"));

            assert_eq!(l, src, "error: {src:?}");
        }
    }

    #[test]
    fn test_full_parser() {
        use super::CssDataType;
        let test_cases = vec![
            ("3", CssDataType::Integer("3".to_string())),
            ("3.14", CssDataType::Number("3.14".to_string())),
            ("23.134", CssDataType::Number("23.134".to_string())),
            ("23.134fr", CssDataType::Fr("23.134".to_string())),
            ("-6px", CssDataType::Length(Length::px("-6".to_string()))),
            (
                "120vmin",
                CssDataType::Length(Length::vmin("120".to_string())),
            ),
            ("red", CssDataType::Color(Color::Named("red".to_string()))),
            ("1px", CssDataType::Length(Length::px("1".to_string()))),
            ("8rem", CssDataType::Length(Length::rem("8".to_string()))),
            ("14rem", CssDataType::Length(Length::rem("14".to_string()))),
            ("1/3", CssDataType::Ratio("1".to_string(), "3".to_string())),
            ("100%", CssDataType::Percentage("100".to_string())),
            ("xx-small", CssDataType::AbsoluteSize(AbsoluteSize::XxSmall)),
            (
                "34.5deg",
                CssDataType::Angle(Angle::Deg("34.5".to_string())),
            ),
            ("center", CssDataType::Position("center".to_string())),
            ("left", CssDataType::Position("left".to_string())),
            (
                "center top",
                CssDataType::Position("center top".to_string()),
            ),
            (
                "right 8.5%",
                CssDataType::Position("right 8.5%".to_string()),
            ),
            (
                "bottom 12vmin right -6px",
                CssDataType::Position("bottom 12vmin right -6px".to_string()),
            ),
            ("10% 20%", CssDataType::Position("10% 20%".to_string())),
            ("8rem 14px", CssDataType::Position("8rem 14px".to_string())),
        ];

        for (src, expected) in test_cases {
            let l = data_type_parser()
                .parse(src)
                .into_result()
                .expect(&format!("full parse error {src:?}"));

            if src.starts_with("bottom") {
                dbg!(&l);
            }

            assert_eq!(l, expected, "error: {src:?}");
        }
    }
}
