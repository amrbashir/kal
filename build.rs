fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        embed_manifest::embed_manifest_file("app.manifest").expect("unable to embed manifest file");
        println!("cargo:rerun-if-changed=app.manifest");
    }
}
