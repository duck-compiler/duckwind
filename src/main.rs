use clap::Parser;
use duckwind::EmitEnv;

use notify::{Event, RecursiveMode, Result, Watcher};
use std::{path::Path, sync::mpsc, time::Instant};

#[derive(Parser, Debug)]
struct Args {
    #[arg(name = "input", help = "load the input from this file")]
    in_file: Vec<String>,
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
    #[arg(
        long,
        short = 'w',
        help = "watch a file, relaunching with the same parameters if it changes (requires out file)"
    )]
    watch: Option<String>,
}

fn main() -> Result<()> {
    let cli = Args::parse();

    let run = || {
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

        let txt = if cli.from_string {
            cli.in_file.clone()
        } else {
            cli.in_file
                .iter()
                .map(|file_name| {
                    std::fs::read_to_string(file_name).expect("Could not read input file")
                })
                .collect()
        };

        for txt in txt {
            let mut i = 0;
            while i < txt.len() {
                if let Some((_, skip)) = emit_env.parse_tailwind_str(&txt[i..]) {
                    i += skip;
                }
                i += 1;
            }
        }

        let as_css = emit_env.to_css_stylesheet(!cli.no_preflight);

        if let Some(out) = cli.out.as_ref() {
            std::fs::write(out, as_css).expect("Could not write output file");
        } else {
            println!("{as_css}");
        }
    };

    if let Some(watch) = cli.watch.as_ref() {
        if cli.out.is_none() {
            println!("error: watch requires out file");
            return Ok(());
        }
        let (tx, rx) = mpsc::channel::<Result<Event>>();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(Path::new(watch.as_str()), RecursiveMode::Recursive)?;
        run();
        println!("Watching... (Ctrl+C to exit)");
        for _ in rx {
            let inst = Instant::now();
            run();
            println!("Recompiled in {}ms.", inst.elapsed().as_millis());
        }
    } else {
        run();
    }

    Ok(())
}
