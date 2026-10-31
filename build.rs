fn main() {
    println!("cargo:rerun-if-changed=resources/pinwin.rc");
    println!("cargo:rerun-if-changed=resources/pinwin.manifest");
    println!("cargo:rerun-if-changed=assets/icons/app.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let version = std::env::var("CARGO_PKG_VERSION").expect("package version");
        let numbers = format!("{},0", version.replace('.', ","));
        let definitions = [
            format!("PINWIN_VERSION={numbers}"),
            format!("PINWIN_VERSION_STR=\"{version}\""),
        ];
        embed_resource::compile(
            "resources/pinwin.rc",
            definitions.iter().map(String::as_str),
        )
        .manifest_required()
        .expect("Windows resources must compile");
    }
}
