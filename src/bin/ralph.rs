const MESSAGE: &str = "ralph is now called CueLoop.\n\nInstall the cueloop crate and use that instead:\n  cargo install cueloop\n\nFor more information, see:\n  https://github.com/fitchmultz/cueloop\n";

fn main() {
    let exit_code = match std::env::args().nth(1).as_deref() {
        Some("--help" | "-h") => {
            print!("{MESSAGE}");
            0
        }
        Some("--version" | "-V") => {
            println!("ralph-agent-loop 0.5.0 (deprecated; use cueloop)");
            0
        }
        _ => {
            eprint!("{MESSAGE}");
            1
        }
    };
    std::process::exit(exit_code);
}
