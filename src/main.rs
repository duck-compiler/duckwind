use duckwind::EmitEnv;

fn main() {
    let mut emit_env = EmitEnv::new_with_default_config();
    let file_name = std::env::args().nth(1).expect("no file name provided");
    let txt = std::fs::read_to_string(&file_name).unwrap();
    for part in txt.split(&[' ', '"', '\'', '\n', '\t']) {
        emit_env.parse_tailwind_str(part);
    }

    std::fs::write("out.css", emit_env.to_css_stylesheet()).unwrap();
}
