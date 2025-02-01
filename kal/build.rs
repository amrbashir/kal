use std::path::Path;

fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        embed_resource::compile("kal.rc", embed_resource::NONE);
        println!("cargo:rerun-if-changed=kal.rc");
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("$CARGO_MANIFEST_DIR not set");
    let manifest_dir = Path::new(&manifest_dir);
    let path = manifest_dir.join("../kal-config/schema.json");

    let schema = schemars::schema_for!(kal_config::Config);
    let schema = serde_json::to_string(&schema).expect("failed to serialize schema into JSON");

    std::fs::write(path, schema).expect("failed to write schema");
}
