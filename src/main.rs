fn main() {
    if let Err(err) = chalkak::run() {
        eprintln!("ChalKak failed: {err}");
        std::process::exit(1);
    }
}
