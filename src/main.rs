fn main() {
    if let Some(code) = handle_early_args() {
        std::process::exit(code);
    }

    if let Err(err) = chalkak::run() {
        eprintln!("ChalKak failed: {err}");
        std::process::exit(1);
    }
}

fn handle_early_args() -> Option<i32> {
    let arg = std::env::args().nth(1)?;
    match arg.as_str() {
        "-V" => {
            println!("ChalKak {}", env!("CARGO_PKG_VERSION"));
            Some(0)
        }
        "--version" => {
            let git_hash = env!("GIT_HASH");
            if git_hash.is_empty() {
                println!("ChalKak {}", env!("CARGO_PKG_VERSION"));
            } else {
                println!("ChalKak {} ({git_hash})", env!("CARGO_PKG_VERSION"));
            }
            Some(0)
        }
        "-h" | "--help" => {
            print_help();
            Some(0)
        }
        _ => None,
    }
}

fn print_help() {
    println!(
        "\
ChalKak — Hyprland screenshot preview and editor utility

Usage: chalkak [OPTIONS]

Options:
  --full, --capture-full        Start with full screen capture
  --region, --capture-region    Start with region capture
  --window, --capture-window    Start with window capture
  --launchpad                   Show the launchpad
  -V                            Print version
  --version                     Print version (with build info)
  -h, --help                    Print this help message"
    );
}
