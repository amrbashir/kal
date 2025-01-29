fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        embed_resource::compile("kal.rc", embed_resource::NONE);
        println!("cargo:rerun-if-changed=kal.rc");
    }
}
