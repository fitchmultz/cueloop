const MESSAGE: &str = "This deprecated ralph-agent-loop package no longer provides CueLoop.\n\nInstall the active cueloop crate instead:\n  cargo install cueloop\n\nFor more information, see:\n  https://github.com/fitchmultz/cueloop\n";

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
