fn main() {
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/chalkak.gresource.xml",
        "chalkak.gresource",
    );

    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();
    let git_hash = output
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
    println!("cargo:rerun-if-changed=.git/HEAD");
}
