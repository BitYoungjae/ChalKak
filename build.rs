fn main() {
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/chalkak.gresource.xml",
        "chalkak.gresource",
    );
}
