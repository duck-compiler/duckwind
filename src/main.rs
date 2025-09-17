use clap::Parser;
use duckwind::EmitEnv;

#[derive(Parser, Debug)]
struct Args {
    #[arg(name = "input", help = "load the input from this file")]
    in_file: String,
    #[arg(
        long = "str",
        short = 's',
        help = "interpret the input as a string and not as a file name"
    )]
    from_string: bool,
    #[arg(
        long,
        short = 'o',
        name = "output file",
        help = "write the output to this file"
    )]
    out: Option<String>,
    #[arg(long, short = 'd', help = "do not include preflight styles")]
    no_preflight: bool,
    #[arg(long, short = 'n', help = "do not load default config")]
    no_default_config: bool,
    #[arg(long, short = 'c', help = "load this config")]
    config: Vec<String>,
}

fn main() {
    let cli = Args::parse();

    let mut emit_env = if cli.no_default_config {
        EmitEnv::new()
    } else {
        EmitEnv::new_with_default_config()
    };

    for config_to_load in &cli.config {
        let config_src = std::fs::read_to_string(config_to_load.as_str())
            .expect(&format!("couldn't load config {config_to_load}"));
        emit_env.load_config(&config_src);
    }

    let file_name = std::env::args().nth(1).expect("no file name provided");
    let txt = if cli.from_string {
        cli.in_file
    } else {
        std::fs::read_to_string(&file_name).expect("Could not read input file")
    };
    let mut i = 0;
    while i < txt.len() {
        if let Some((_, skip)) = emit_env.parse_tailwind_str(&txt[i..]) {
            i += skip;
        }
        i += 1;
    }

    let as_css = emit_env.to_css_stylesheet(!cli.no_preflight);

    if let Some(out) = cli.out {
        std::fs::write(out, as_css).expect("Could not write output file");
    } else {
        println!("{as_css}");
    }
}
