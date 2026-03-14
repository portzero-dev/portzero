use std::fs;
use std::path::Path;

fn main() {
    // rust-embed requires the folder to exist at compile time.
    // When dashboard-dist/ hasn't been pre-built (e.g. CI without a
    // frontend build step), create a minimal placeholder so the
    // crate still compiles.
    let dashboard_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dashboard-dist");

    if !dashboard_dir.exists() {
        fs::create_dir_all(&dashboard_dir).expect("failed to create dashboard-dist");
    }

    let index = dashboard_dir.join("index.html");
    if !index.exists() {
        fs::write(
            &index,
            "<!DOCTYPE html><html><head><title>PortZero</title></head><body>\
             <p>PortZero dashboard not built yet. \
             Run the frontend build to populate this folder.</p></body></html>\n",
        )
        .expect("failed to write placeholder index.html");
    }

    println!("cargo::rerun-if-changed=../../dashboard-dist");
}
